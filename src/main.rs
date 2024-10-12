#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
mod window;

#[cfg(target_arch = "wasm32")]
mod wasm_window;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()>
{
    eframe::run_native(
        "Filler",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok( Box::new(window::Window::new()) ))
    )
}

#[cfg(target_arch = "wasm32")]
fn main()
{
    use eframe::wasm_bindgen::JsCast;

    wasm_bindgen_futures::spawn_local(async {
        let document = eframe::web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<eframe::web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new( |_cc| Ok(Box::new( wasm_window::WasmWindow::new()) )),
            )
            .await;

        if let Some(loading_text) = document.get_element_by_id("loading_text")
        {
            match start_result
            {
                Ok(_) => {
                    loading_text.remove();
                },
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                },
            }
        }
    });
}