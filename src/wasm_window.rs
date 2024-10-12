use std::sync::mpsc::{Sender, Receiver, channel};
use eframe::Frame;
use egui::Context;

pub struct WasmWindow
{
    topics_channel: (Sender<String>, Receiver<String>),
    topics: Vec<String>,
    active_topics_channel: (Sender<String>, Receiver<String>),
    active_topics: Vec<bool>,
    summaries_channel: (Sender<String>, Receiver<String>),
    summaries: Vec<String>,
    active_summary: i32,
    new_summary_channel: (Sender<String>, Receiver<String>),
    topic_choices_channel: (Sender<String>, Receiver<String>),
    topic_choices: Vec<Vec<String>>,
    chosen_topic: String,
}

impl WasmWindow
{
    pub fn new() -> WasmWindow
    {
        WasmWindow {
            topics_channel: channel(),
            topics: Vec::new(),
            active_topics_channel: channel(),
            active_topics: Vec::new(),
            summaries_channel: channel(),
            summaries: Vec::new(),
            active_summary: -1,
            new_summary_channel: channel(),
            topic_choices_channel: channel(),
            topic_choices: Vec::new(),
            chosen_topic: String::new(),
        }
    }

    async fn create_topics(content: String, sender: Sender<String>)
    {
        let mut topics = Vec::<String>::new();
        for line in content.lines()
        {
            let line = line.trim();
            if !line.is_empty()
            {
                topics.push(line.to_string());
            }
        }
        sender.send(topics.join("\n")).unwrap();
    }

    async fn learn_topics(topics: String, active_topics: String, sender_summaries: Sender<String>, sender_choices: Sender<String>, sender_active_topics: Sender<String>)
    {
        let topics = serde_json::from_str::<Vec<String>>(&topics).unwrap();
        let mut active_topics = serde_json::from_str::<Vec<bool>>(&active_topics).unwrap();
        let mut summaries = Vec::<String>::new();
        let mut choices = Vec::<Vec<String>>::new();

        let wiki = wikipedia_wasm::Wikipedia::<wikipedia_wasm::http::default::Client>::default();

        for (i, topic) in topics.iter().enumerate()
        {
            if active_topics[i]
            {
                if let Ok(results) = wiki.search(topic.as_str()).await
                {
                    if results.is_empty()
                    {
                        active_topics[i] = false;
                        let _ = sender_active_topics.send(serde_json::to_string(&active_topics).unwrap());
                        continue;
                    }

                    let page = wiki.page_from_title(results[0].clone());

                    choices.push(results);
                    if let Ok(summary) = page.get_summary().await
                    {
                        summaries.push(summary);
                        let _ = sender_summaries.send(serde_json::to_string(&summaries).unwrap());
                        let _ = sender_choices.send(serde_json::to_string(&choices).unwrap());
                    }
                }
            }
        }
    }

    async fn create_summary(topic: String, sender: Sender<String>)
    {
        let wiki = wikipedia_wasm::Wikipedia::<wikipedia_wasm::http::default::Client>::default();
        let page = wiki.page_from_title(topic);
        if let Ok(summary) = page.get_summary().await
        {
            let _ = sender.send(summary);
        }
        else
        {
            let _ = sender.send(String::new());
        }
    }
}

impl eframe::App for WasmWindow
{
    fn update(&mut self, ctx: &Context, _frame: &mut Frame)
    {
        if let Ok(topics_recv) = self.topics_channel.1.try_recv()
        {
            self.topics = topics_recv.lines().map(|line| line.to_string()).collect();
            self.active_topics.resize(self.topics.len(), true);
            self.summaries.clear();
            self.active_summary = -1;
            self.topic_choices.clear();
            self.chosen_topic.clear();
        }
        if let Ok(active_topics_recv) = self.active_topics_channel.1.try_recv()
        {
            self.active_topics = serde_json::from_str(active_topics_recv.as_str()).unwrap();
        }
        if let Ok(summaries_recv) = self.summaries_channel.1.try_recv()
        {
            self.summaries = serde_json::from_str(summaries_recv.as_str()).unwrap();
        }
        if let Ok(new_summary_recv) = self.new_summary_channel.1.try_recv()
        {
            self.summaries[self.active_summary as usize] = new_summary_recv;
        }
        if let Ok(choices_recv) = self.topic_choices_channel.1.try_recv()
        {
            self.topic_choices = serde_json::from_str(choices_recv.as_str()).unwrap();
        }

        egui::CentralPanel::default().show(ctx, |ui|{
            ui.horizontal(|ui| {
                if ui.button("Open File").clicked()
                {
                    let sender = self.topics_channel.0.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Some(file) = rfd::AsyncFileDialog::new()
                            .add_filter("text", &["txt"])
                            .set_directory("/")
                            .pick_file().await
                        {
                            if let Ok(content) = String::from_utf8(file.read().await)
                            {
                                WasmWindow::create_topics(content, sender).await;
                            }
                        }
                    });
                }
                if ui.button("Get Summaries").clicked()
                {
                    if !self.topics.is_empty()
                    {
                        let topics = serde_json::to_string(&self.topics).unwrap();
                        let active_topics = serde_json::to_string(&self.active_topics).unwrap();
                        let sender_summaries = self.summaries_channel.0.clone();
                        let sender_choices = self.topic_choices_channel.0.clone();
                        let sender_active_topics = self.active_topics_channel.0.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            Self::learn_topics(topics, active_topics, sender_summaries, sender_choices, sender_active_topics).await;
                        });
                    }
                }
                if ui.button("Create File").clicked()
                {
                    if !self.summaries.is_empty() && self.summaries.len() == self.active_topics.iter().filter(|x| **x).count()
                    {
                        let topics = self.topics.clone();
                        let active_topics = self.active_topics.clone();
                        let summaries = self.summaries.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            if let Some(file) = rfd::AsyncFileDialog::new()
                                .add_filter("text", &["txt"])
                                .set_directory("/")
                                .set_file_name("out.txt")
                                .save_file().await
                            {
                                let mut content = Vec::<u8>::new();
                                let mut summary_i = 0usize;
                                for (i, topic) in topics.iter().enumerate()
                                {
                                    for b in format!("{}\n", topic).bytes()
                                    {
                                        content.push(b);
                                    }
                                    if active_topics[i]
                                    {
                                        for b in format!("{}\n\n", summaries[summary_i]).bytes()
                                        {
                                            content.push(b);
                                        }
                                        summary_i += 1;
                                    }
                                    else
                                    {
                                        for b in "\n\n".bytes()
                                        {
                                            content.push(b);
                                        }
                                    }
                                }
                                let b = content.as_slice();
                                file.write(b).await.unwrap();
                            }
                        });
                    }
                }
            });

            ui.add_space(10.0);

            if self.summaries.is_empty()
            {
                egui::ScrollArea::both().max_width(238.0).show(ui, |ui| {
                    egui::Grid::new("Topics").show(ui, |ui| {
                        for (i, topic) in self.topics.iter().enumerate()
                        {
                            ui.checkbox(&mut self.active_topics[i], topic);
                            ui.end_row();
                        }
                    });
                });
            }
            else
            {
                ui.horizontal_top(|ui| {
                    let width = ui.push_id(420, |ui| {
                        egui::ScrollArea::both().max_width(450.0).show(ui, |ui| {
                            egui::Grid::new("Summaries").show(ui, |ui| {
                                let mut summary_i = 0usize;
                                for (i, topic) in self.topics.iter().enumerate()
                                {
                                    if self.active_topics[i] && summary_i < self.summaries.len()
                                    {
                                        if let Some(s) = self.summaries.get(summary_i)
                                        {
                                            if s.is_empty()
                                            {
                                                continue;
                                            }
                                            if ui.button(topic).clicked()
                                            {
                                                self.active_summary = summary_i as i32;
                                                self.chosen_topic = self.topic_choices[self.active_summary as usize][0].clone();
                                            }
                                            ui.end_row();
                                            summary_i += 1;
                                        }
                                    }
                                }
                            });
                        });
                    }).response.rect.width();

                    if width < 238.0
                    {
                        ui.add_space(238.0 - width);
                    }

                    ui.vertical(|ui| {
                        if self.active_summary != -1
                        {
                            egui::ComboBox::from_label("")
                                .selected_text(&self.chosen_topic)
                                .show_ui(ui, |ui| {
                                    for choice in self.topic_choices[self.active_summary as usize].iter()
                                    {
                                        if ui.selectable_value(&mut self.chosen_topic, choice.clone(), choice).clicked()
                                        {
                                            let chosen_topic = self.chosen_topic.clone();
                                            let sender = self.new_summary_channel.0.clone();
                                            wasm_bindgen_futures::spawn_local(async move {
                                                Self::create_summary(chosen_topic, sender).await;
                                            });
                                        }
                                    }
                                });

                            egui::ScrollArea::vertical().show(ui, |ui|{
                                ui.add(egui::Label::new(&self.summaries[self.active_summary as usize]).wrap());
                            });
                        }
                    });
                });

            }
        });
    }
}