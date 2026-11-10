//! Simple Rust Bible Reader desktop shell: the Bible tab of
//! `RendererWebView2` on top of `wry`/`tao`.

use sbr_core::bible_format::{self, Bible, FILTERS};
use sbr_core::{global_memory, render};
use serde::Deserialize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::window::WindowBuilder;
use wry::{DragDropEvent, WebView, WebViewBuilder};

const UI_HTML: &str = include_str!("ui.html");

/// Lets the chapter iframe use the WebView2 `chrome.webview.postMessage`
/// calls emitted by `getChapterHtml`.
const CHAPTER_SHIM: &str = concat!(
    "<script>window.chrome={webview:{postMessage:function(m){parent.postMessage({sbr:String(m)},'*')}}};",
    "addEventListener('message',function(e){var m=e.data;if(e.source!==parent||!m||typeof m.selectVerse!='string')return;",
    "for(var a of document.getElementsByTagName('a'))a.className=m.para?'para':'linear';",
    "var v=document.getElementById(m.selectVerse);if(v)v.className='selectedverse';});</script>"
);

#[derive(Deserialize)]
#[serde(tag = "cmd", rename_all = "lowercase")]
enum Command {
    Ready,
    Open,
    Export,
    Chapter { book: String, chapter: i32 },
    Para,
    Raw,
    Theme { name: String },
    WebMessage { message: String },
}

enum UserEvent {
    Ipc(String),
    Open(String),
    Loaded(String, Result<Bible, String>),
    Exported(String, Result<(), String>),
}

#[derive(Default)]
struct App {
    bible: Option<Bible>,
    current: Option<(String, i32)>,
    para: bool,
    busy: bool,
}

fn js<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).expect("serializable")
}

fn bible_extensions() -> Vec<String> {
    let mut v: Vec<String> = FILTERS
        .iter()
        .flat_map(|(_, p)| p.iter().map(|e| e.trim_start_matches("*.").to_string()))
        .collect();
    v.sort();
    v.dedup();
    v
}

impl App {
    fn eval(&self, wv: &WebView, script: &str) {
        if let Err(e) = wv.evaluate_script(script) {
            eprintln!("evaluate_script: {e}");
        }
    }

    fn status(&self, wv: &WebView, text: &str) {
        self.eval(wv, &format!("sbr.status({})", js(&text)));
    }

    fn open(&mut self, wv: &WebView, proxy: &EventLoopProxy<UserEvent>, path: String) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.status(wv, &format!("Loading {path} ..."));
        let proxy = proxy.clone();
        std::thread::spawn(move || {
            let r = bible_format::load(&path, 0).map_err(|e| e.to_string());
            let _ = proxy.send_event(UserEvent::Loaded(path, r));
        });
    }

    fn loaded(&mut self, wv: &WebView, path: &str, r: Result<Bible, String>) {
        self.busy = false;
        let mut b = match r {
            Ok(b) => b,
            Err(e) => return self.status(wv, &format!("Could not open {path}: {e}")),
        };
        let tree = match render::populate_books(&mut b) {
            Ok(t) => t,
            Err(e) => return self.status(wv, &format!("Could not read books: {e}")),
        };
        let title = if b.description.is_empty() {
            b.abbreviation.clone()
        } else {
            b.description.clone()
        };
        self.eval(
            wv,
            &format!("sbr.setBooks({}, {})", js(&title), js(&tree.groups())),
        );
        let first = tree
            .groups()
            .into_iter()
            .flat_map(|(_, books)| books.iter())
            .find_map(|(n, c)| c.first().map(|&c| (n.clone(), c)));
        self.bible = Some(b);
        self.current = None;
        match first {
            Some((book, ch)) => self.show(wv, &book, ch),
            None => self.status(wv, &format!("{path}: no books")),
        }
    }

    fn show(&mut self, wv: &WebView, book: &str, chapter: i32) {
        let Some(b) = self.bible.as_mut() else { return };
        match render::chapter_html(b, book, &chapter.to_string(), self.para, None) {
            Ok(html) => {
                let html = html.replacen("<head>", &format!("<head>{CHAPTER_SHIM}"), 1);
                self.current = Some((book.to_string(), chapter));
                self.eval(
                    wv,
                    &format!("sbr.showChapter({}, {}, {chapter})", js(&html), js(&book)),
                );
            }
            Err(e) => self.status(wv, &format!("{book} {chapter}: {e}")),
        }
    }

    fn refresh(&mut self, wv: &WebView) {
        if let Some((book, ch)) = self.current.clone() {
            self.show(wv, &book, ch);
        }
    }

    fn init_js(&self) -> String {
        let theme = global_memory::get().selected_theme.clone();
        format!(
            "sbr.init({{themes: {}, theme: {}}}); sbr.setToggle('para', {}); sbr.setToggle('raw', {})",
            js(&render::themes()),
            js(&theme),
            self.para,
            global_memory::get().display_bib_raw
        )
    }

    fn export(&mut self, wv: &WebView, proxy: &EventLoopProxy<UserEvent>) {
        if self.busy || self.bible.is_none() {
            return self.status(wv, "Open a Bible first");
        }
        let mut dlg = rfd::FileDialog::new().set_title("Export Bible");
        for (name, pats) in FILTERS {
            let exts: Vec<&str> = pats.iter().map(|p| p.trim_start_matches("*.")).collect();
            dlg = dlg.add_filter(*name, &exts);
        }
        let Some(path) = dlg.save_file() else { return };
        let path = path.to_string_lossy().into_owned();
        // rfd does not report the chosen filter, so pick the format from the
        // extension (first filter that lists it), as `detect` would.
        let lower = path.to_lowercase();
        let ext = lower.rsplit('.').next().unwrap_or("").to_string();
        let Some(idx) = FILTERS.iter().position(|(_, p)| {
            p.iter()
                .any(|e| lower.ends_with(&e.trim_start_matches('*').to_lowercase()))
        }) else {
            return self.status(wv, &format!("Unknown export format: .{ext}"));
        };
        let mut b = self.bible.take().expect("checked");
        self.busy = true;
        self.status(wv, &format!("Exporting {path} as {} ...", FILTERS[idx].0));
        let proxy = proxy.clone();
        std::thread::spawn(move || {
            let r = bible_format::export(&mut b, &path, idx as i32 + 1).map_err(|e| e.to_string());
            let _ = proxy.send_event(UserEvent::Loaded(String::new(), Ok(b)));
            let _ = proxy.send_event(UserEvent::Exported(path, r));
        });
    }

    fn command(&mut self, wv: &WebView, proxy: &EventLoopProxy<UserEvent>, cmd: Command) {
        match cmd {
            Command::Ready => {
                self.eval(wv, &self.init_js());
                if let Some(path) = std::env::args().nth(1) {
                    self.open(wv, proxy, path);
                }
            }
            Command::Open => {
                let exts = bible_extensions();
                if let Some(p) = rfd::FileDialog::new()
                    .set_title("Open Bible")
                    .add_filter("Bibles", &exts)
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    self.open(wv, proxy, p.to_string_lossy().into_owned());
                }
            }
            Command::Export => self.export(wv, proxy),
            Command::Chapter { book, chapter } => self.show(wv, &book, chapter),
            Command::Para => {
                self.para = !self.para;
                self.eval(wv, &format!("sbr.setToggle('para', {})", self.para));
                self.refresh(wv);
            }
            Command::Raw => {
                let on = {
                    let mut g = global_memory::get_mut();
                    g.display_bib_raw = !g.display_bib_raw;
                    g.display_bib_raw
                };
                self.eval(wv, &format!("sbr.setToggle('raw', {on})"));
                self.refresh(wv);
            }
            Command::Theme { name } => {
                global_memory::get_mut().selected_theme = name;
                self.eval(wv, &self.init_js());
                self.refresh(wv);
            }
            Command::WebMessage { message } => self.web_message(wv, &message),
        }
    }

    /// `ChapterArea_WebMessageReceived`.
    fn web_message(&mut self, wv: &WebView, message: &str) {
        let message = message.trim();
        let parts: Vec<&str> = message.split('-').collect();
        if parts.len() == 3 {
            let Some(b) = self.bible.as_ref() else { return };
            let book = parts[0].parse().unwrap_or(0);
            self.eval(
                wv,
                &format!("sbr.selectVerse({}, {})", js(&parts[2]), self.para),
            );
            self.status(
                wv,
                &format!("{} {}:{}", render::book_name(b, book), parts[1], parts[2]),
            );
        } else if let Some(lex) = message.strip_prefix('L') {
            let lex: String = lex
                .chars()
                .filter(|c| matches!(c, 'H' | 'G' | '0'..='9'))
                .collect();
            self.status(
                wv,
                &format!("Strong's {lex} (open a dictionary to look it up)"),
            );
        }
    }
}

fn main() -> wry::Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let window = WindowBuilder::new()
        .with_title("Simple Bible Reader")
        .with_inner_size(tao::dpi::LogicalSize::new(1100.0, 750.0))
        .build(&event_loop)
        .expect("create window");

    let ipc_proxy = proxy.clone();
    let drop_proxy = proxy.clone();
    let builder = WebViewBuilder::new()
        .with_html(UI_HTML)
        .with_devtools(cfg!(debug_assertions))
        .with_ipc_handler(move |req| {
            let _ = ipc_proxy.send_event(UserEvent::Ipc(req.into_body()));
        })
        .with_drag_drop_handler(move |e| {
            if let DragDropEvent::Drop { paths, .. } = e {
                if let Some(p) = paths.first() {
                    let _ =
                        drop_proxy.send_event(UserEvent::Open(p.to_string_lossy().into_owned()));
                }
            }
            true
        });

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let webview = builder.build(&window)?;
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        builder.build_gtk(window.default_vbox().expect("gtk vbox"))?
    };

    let mut app = App::default();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::UserEvent(UserEvent::Ipc(body)) => match serde_json::from_str(&body) {
                Ok(cmd) => app.command(&webview, &proxy, cmd),
                Err(e) => eprintln!("bad ipc message {body:?}: {e}"),
            },
            Event::UserEvent(UserEvent::Open(path)) => app.open(&webview, &proxy, path),
            Event::UserEvent(UserEvent::Loaded(path, r)) => {
                if path.is_empty() {
                    // Bible handed back after an export.
                    app.bible = r.ok();
                } else {
                    app.loaded(&webview, &path, r);
                }
            }
            Event::UserEvent(UserEvent::Exported(path, r)) => {
                app.busy = false;
                match r {
                    Ok(()) => app.status(&webview, &format!("Exported {path}")),
                    Err(e) => app.status(&webview, &format!("Export to {path} failed: {e}")),
                }
            }
            _ => {}
        }
    });
}
