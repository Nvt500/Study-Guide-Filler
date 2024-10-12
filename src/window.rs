use std::fs;
use std::io::{Read, Seek, Write};
use std::path::PathBuf;
use eframe::Frame;
use egui::{Context};
use rfd::FileDialog;

pub struct Window
{
    topics: Vec<String>,
    active_topics: Vec<bool>,
    summaries: Vec<String>,
    active_summary: i32,
    topic_choices: Vec<Vec<String>>,
    chosen_topic: String,
    output_file_path: PathBuf,
}

impl Window
{
    pub fn new() -> Window
    {
        Window {
            topics: Vec::new(),
            active_topics: Vec::new(),
            summaries: Vec::new(),
            active_summary: -1,
            topic_choices: Vec::new(),
            chosen_topic: String::new(),
            output_file_path: PathBuf::new(),
        }
    }

    fn create_topics(&mut self, mut file: fs::File)
    {
        let mut content = String::new();
        if file.read_to_string(&mut content).is_ok()
        {
            self.topics.clear();
            self.active_topics.clear();
            self.summaries.clear();
            self.active_summary = -1;
            self.topic_choices.clear();
            self.chosen_topic.clear();
            for line in content.lines()
            {
                let line = line.trim();
                if !line.is_empty()
                {
                    self.topics.push(line.into());
                    self.active_topics.push(true);
                }
            }
        }
    }

    fn learn_topics(&mut self)
    {
        let wiki = wikipedia::Wikipedia::<wikipedia::http::default::Client>::default();

        for (i, topic) in self.topics.iter().enumerate()
        {
            if self.active_topics[i]
            {
                if let Ok(results) = wiki.search(topic.as_str())
                {
                    if results.is_empty()
                    {
                        self.active_topics[i] = false;
                        continue;
                    }

                    let page = wiki.page_from_title(results[0].clone());

                    self.topic_choices.push(results);
                    if let Ok(summary) = page.get_summary()
                    {
                        self.summaries.push(summary);
                    }
                }
            }
        }
    }

    fn create_summary(&self, title: String) -> String
    {
        let wiki = wikipedia::Wikipedia::<wikipedia::http::default::Client>::default();
        let page = wiki.page_from_title(title);
        if let Ok(summary) = page.get_summary()
        {
            summary
        }
        else
        {
            String::new()
        }
    }

    fn write_summaries(&self, path: PathBuf)
    {
        let mut file = fs::File::options().write(true).create(true).open(path).unwrap();
        file.set_len(0).unwrap();
        file.rewind().unwrap();
        let summary_i = 0usize;
        for (i, topic) in self.topics.iter().enumerate()
        {
            file.write(format!("{}\n", topic).as_bytes()).unwrap();
            if self.active_topics[i]
            {
                file.write(format!("{}\n\n", self.summaries[summary_i]).as_bytes()).unwrap();
            }
            else
            {
                file.write("\n\n".as_bytes()).unwrap();
            }
        }
    }
}

impl eframe::App for Window
{
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open File").clicked()
                {
                    let files = FileDialog::new()
                        .add_filter("text", &["txt"])
                        .set_directory("/")
                        .pick_file();
                    if files.is_some()
                    {
                        let files = files.unwrap();
                        if let Ok(file) = fs::File::options()
                            .read(true)
                            .create(false)
                            .create_new(false)
                            .open(&files)
                        {
                            self.output_file_path = files.with_file_name("out.txt");
                            self.create_topics(file);
                        }
                    }
                }
                if ui.button("Get Summaries").clicked()
                {
                    if !self.topics.is_empty()
                    {
                        self.learn_topics();
                    }
                }
                if ui.button("Create File").clicked()
                {
                    if !self.summaries.is_empty() && self.summaries.len() == self.active_topics.iter().filter(|x| **x).count()
                    {
                        if let Some(path) = FileDialog::new()
                            .add_filter("text", &["txt"])
                            .set_file_name("out")
                            .set_directory("/")
                            .save_file()
                        {
                            self.write_summaries(path);
                        }
                    }
                }
            });

            ui.add_space(10.0);

            if !self.topics.is_empty()
            {
                if self.summaries.is_empty()
                {
                    ui.push_id(69, |ui| {
                        egui::ScrollArea::both().max_width(238.0).show(ui, |ui| {
                            egui::Grid::new("Topics").show(ui, |ui| {
                                for (i, topic) in self.topics.iter().enumerate()
                                {
                                    ui.checkbox(&mut self.active_topics[i], topic);
                                    ui.end_row();
                                }
                            });
                        });
                    });
                }
                else
                {
                    ui.horizontal_top(|ui| {
                        let width = ui.push_id(420, |ui| {
                            egui::ScrollArea::both().max_width(238.0).show(ui, |ui| {
                                egui::Grid::new("Topics").show(ui, |ui| {
                                    let mut summary_i = 0usize;
                                    for (i, topic) in self.topics.iter().enumerate()
                                    {
                                        if self.active_topics[i]
                                        {
                                            if ui.button(topic).clicked()
                                            {
                                                self.active_summary = summary_i as i32;
                                                self.chosen_topic = self.topic_choices[self.active_summary as usize][0].clone();
                                            }
                                            ui.end_row();
                                            summary_i += 1;
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
                                        for choice in &self.topic_choices[self.active_summary as usize]
                                        {
                                            if ui.selectable_value(&mut self.chosen_topic, choice.clone(), choice).clicked()
                                            {
                                                self.summaries[self.active_summary as usize] = self.create_summary(self.chosen_topic.clone());
                                            }
                                        }
                                    }
                                );

                                egui::ScrollArea::vertical().show(ui, |ui|{
                                    ui.add(egui::Label::new(&self.summaries[self.active_summary as usize]).wrap());
                                });
                            }
                        });
                    });
                }
            }
        });

    }
}