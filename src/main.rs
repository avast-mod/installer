#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod install;

use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, RichText};
use install::{Build, Game};

const ICON: &[u8] = include_bytes!("../assets/icon.png");
const SPLASH: &[u8] = include_bytes!("../assets/splash.png");
const FONT_BODY: &[u8] = include_bytes!("../assets/fonts/fs-tahoma-8px.ttf");
const FONT_BODY_BOLD: &[u8] = include_bytes!("../assets/fonts/fs-tahoma-8px-bold.ttf");
const FONT_NARROW: &[u8] = include_bytes!("../assets/fonts/RobotoCondensed-Regular.ttf");
const FONT_NARROW_BOLD: &[u8] = include_bytes!("../assets/fonts/RobotoCondensed-Bold.ttf");

const RED: Color32 = Color32::from_rgb(0xCC, 0x1B, 0x1B);
const LINK: Color32 = Color32::from_rgb(0xC0, 0x10, 0x10);
const TEXT: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);
const PAGE: Color32 = Color32::from_rgb(0xFA, 0xFA, 0xFA);
const BOX_FILL: Color32 = Color32::from_rgb(0xEC, 0xDA, 0xDA);
const BOX_LINE: Color32 = Color32::from_rgb(0x9C, 0x3C, 0x3C);
const FOOTER: Color32 = Color32::from_rgb(0x3C, 0x3C, 0x3C);
const AMBER: Color32 = Color32::from_rgb(0xA8, 0x55, 0x00);
const GREEN: Color32 = Color32::from_rgb(0x1E, 0x7A, 0x2E);

const CREDITS_URL: &str = "https://github.com/avast-mod/installer/blob/main/assets/fonts/README.md";
const STEPS: [&str; 4] = ["welcome", "folder", "install", "done"];

fn main() -> eframe::Result {
    let icon = eframe::icon_data::from_png_bytes(ICON).expect("bundled icon is a valid PNG");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("AVaSt Installer")
            .with_inner_size([640.0, 440.0])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_icon(icon.clone()),
        ..Default::default()
    };
    eframe::run_native(
        "AVaSt Installer",
        options,
        Box::new(move |cc| {
            setup_style(&cc.egui_ctx);
            let splash = load_png(&cc.egui_ctx, "splash", SPLASH, egui::TextureOptions::LINEAR);
            Ok(Box::new(App::new(splash)))
        }),
    )
}

#[derive(PartialEq)]
enum Page {
    Welcome,
    Location,
    Ready,
    Installing,
    Done,
}

#[derive(Default)]
struct Progress {
    lines: Vec<String>,
    result: Option<Result<(), String>>,
}

struct App {
    page: Page,
    splash: egui::TextureHandle,
    found: Vec<Game>,
    path: String,
    game: Option<Game>,
    progress: Arc<Mutex<Progress>>,
}

impl App {
    fn new(splash: egui::TextureHandle) -> Self {
        let found = install::find_games();
        let mut app = Self {
            page: Page::Welcome,
            splash,
            path: String::new(),
            game: None,
            found,
            progress: Arc::default(),
        };
        if let Some(first) = app.found.first() {
            app.set_path(first.dir.display().to_string());
        }
        app
    }

    fn set_path(&mut self, path: String) {
        self.game = install::inspect(std::path::Path::new(path.trim()));
        self.path = path;
    }

    fn start_install(&mut self, ctx: &egui::Context) {
        let Some(game) = self.game.clone() else { return };
        *self.progress.lock().unwrap() = Progress::default();
        self.page = Page::Installing;
        let progress = self.progress.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let log = |line: String| {
                progress.lock().unwrap().lines.push(line);
                ctx.request_repaint();
            };
            let result = install::install(&game, &log);
            progress.lock().unwrap().result = Some(result);
            ctx.request_repaint();
        });
    }

    fn splash(&self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        ui.painter().image(
            self.splash.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    fn step(&self) -> usize {
        match self.page {
            Page::Welcome => 0,
            Page::Location => 1,
            Page::Ready | Page::Installing => 2,
            Page::Done => 3,
        }
    }

    fn page(&mut self, ui: &mut egui::Ui) {
        match self.page {
            Page::Welcome => {
                heading(ui, "Welcome");
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    bold(ui, "AVaSt", TEXT);
                    ui.label(" adds mods to ");
                    link(ui, "Antivirus Survivors 2003 Professional", "https://store.steampowered.com/app/3832490");
                    ui.label(". This installer sets everything up for you.");
                });
                ui.add_space(12.0);
                pink_box(ui, |ui| {
                    bold(ui, "It will:", TEXT);
                    ui.add_space(4.0);
                    bullet(ui, "find your game folder");
                    bullet(ui, "install GDPatch, the mod loader, if it is missing");
                    bullet(ui, "install the latest AVaSt API and AVaSt Core");
                });
                ui.add_space(12.0);
                ui.label("Your save files are not touched. Close the game before you continue.");
            }
            Page::Location => self.location(ui),
            Page::Ready => {
                let Some(game) = &self.game else { return };
                heading(ui, "Ready to install");
                pink_box(ui, |ui| {
                    field(ui, "Game folder", &game.dir.display().to_string());
                    field(ui, "Game build", build_name(game.build));
                    field(
                        ui,
                        "GDPatch",
                        if game.gdpatch_installed() {
                            "Already installed, will be kept"
                        } else {
                            "Latest release will be downloaded"
                        },
                    );
                    field(ui, "Mods", "AVaSt API and AVaSt Core (latest releases)");
                });
                warnings(ui, game);
            }
            Page::Installing | Page::Done => self.progress(ui),
        }
    }

    fn location(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Game folder");
        if self.found.is_empty() {
            ui.label("The game was not found in your Steam libraries. Pick the folder that holds the game files.");
        } else {
            ui.label("Found the game in your Steam library. Is this the right folder?");
        }
        ui.add_space(10.0);

        if self.found.len() > 1 {
            let dirs: Vec<String> = self.found.iter().map(|g| g.dir.display().to_string()).collect();
            for dir in dirs {
                if ui.radio(self.path == dir, &dir).clicked() {
                    self.set_path(dir);
                }
            }
            ui.add_space(6.0);
        }

        ui.horizontal(|ui| {
            let mut path = self.path.clone();
            let field = egui::TextEdit::singleline(&mut path).desired_width(ui.available_width() - 90.0);
            if ui.add(field).changed() {
                self.set_path(path);
            }
            if gray_button(ui, "Browse...", true).clicked()
                && let Some(dir) = rfd::FileDialog::new().set_title("Select the game folder").pick_folder()
            {
                self.set_path(dir.display().to_string());
            }
        });
        ui.add_space(8.0);
        match &self.game {
            Some(game) => ui.colored_label(GREEN, format!("✔ Game found ({})", build_name(game.build))),
            None if self.path.trim().is_empty() => ui.label(""),
            None => ui.colored_label(RED, "✖ No game files in this folder"),
        };
    }

    fn progress(&mut self, ui: &mut egui::Ui) {
        let progress = self.progress.lock().unwrap();
        match &progress.result {
            None => {
                ui.horizontal(|ui| {
                    heading(ui, "Installing...");
                    ui.add(egui::Spinner::new().color(RED));
                });
            }
            Some(Ok(())) => heading(ui, "AVaSt is installed"),
            Some(Err(_)) => heading(ui, "Installation failed"),
        }
        egui::Frame::NONE
            .fill(Color32::WHITE)
            .stroke(egui::Stroke::new(1.0, Color32::from_gray(0xA0)))
            .inner_margin(6)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::vertical()
                    .max_height(100.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in &progress.lines {
                            ui.label(line);
                        }
                    });
            });
        ui.add_space(8.0);

        let Some(game) = &self.game else { return };
        match &progress.result {
            Some(Err(error)) => {
                ui.colored_label(RED, error);
                ui.label("Check your internet connection and that the game is closed, then try again.");
            }
            Some(Ok(())) => match game.launch_options() {
                None => {
                    ui.label("Start the game from Steam as usual. The AVaSt icon appears on the desktop.");
                }
                Some(options) => {
                    bold(ui, "One last step: Steam launch options", TEXT);
                    ui.label("In Steam, right-click the game → Properties → General → Launch Options, and paste:");
                    ui.horizontal(|ui| {
                        let mut text = options.clone();
                        ui.add(egui::TextEdit::singleline(&mut text).desired_width(ui.available_width() - 70.0));
                        if gray_button(ui, "Copy", true).clicked() {
                            ui.ctx().copy_text(options);
                        }
                    });
                    warnings(ui, game);
                }
            },
            None => {}
        }
    }

    fn buttons(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let result = self.progress.lock().unwrap().result.clone();
            match self.page {
                Page::Welcome => {
                    if red_button(ui, "Next  ›", true).clicked() {
                        self.page = Page::Location;
                    }
                }
                Page::Location => {
                    if red_button(ui, "Next  ›", self.game.is_some()).clicked() {
                        self.page = Page::Ready;
                    }
                    if gray_button(ui, "‹  Back", true).clicked() {
                        self.page = Page::Welcome;
                    }
                }
                Page::Ready => {
                    if red_button(ui, "⬇  Install", true).clicked() {
                        self.start_install(ui.ctx());
                    }
                    if gray_button(ui, "‹  Back", true).clicked() {
                        self.page = Page::Location;
                    }
                }
                Page::Installing => {
                    if result.is_some() {
                        self.page = Page::Done;
                    }
                    red_button(ui, "Finish", false);
                }
                Page::Done => {
                    if red_button(ui, "Finish", true).clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    match result {
                        Some(Ok(())) => {
                            if gray_button(ui, "Launch game", true).clicked() {
                                install::open(&format!("steam://rungameid/{}", install::STEAM_APP_ID));
                            }
                            if gray_button(ui, "Open folder", true).clicked()
                                && let Some(game) = &self.game
                            {
                                install::open(&game.gdpatch_dir().display().to_string());
                            }
                        }
                        _ => {
                            if gray_button(ui, "Try again", true).clicked() {
                                self.page = Page::Ready;
                            }
                        }
                    }
                }
            }
            if self.page != Page::Done
                && gray_button(ui, "Cancel", self.page != Page::Installing).clicked()
            {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                link(ui, "Documentation", install::DOCS_URL);
                link(ui, "Credits", CREDITS_URL)
                    .on_hover_text("Fonts: fs Tahoma 8px by ETHproductions and its bold adaptation (CC BY-SA 3.0), Roboto Condensed by The Roboto Project Authors and Michroma by The Michroma Project Authors (SIL OFL 1.1)");
            });
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::bottom("footer")
            .exact_size(22.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::NONE.fill(FOOTER).inner_margin(egui::Margin::symmetric(12, 0)))
            .show(ui, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(
                                "© 2026 malidev. Not affiliated with Shaun Hammond Business Solutions or any other company.",
                            )
                            .color(Color32::from_gray(0xB4)),
                        );
                    });
                });
            });
        egui::Panel::left("splash")
            .exact_size(200.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.splash(ui));
        egui::Panel::bottom("buttons")
            .show_separator_line(false)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(18, 12)).fill(PAGE))
            .show(ui, |ui| {
                let top = ui.max_rect().top() - 12.0;
                ui.painter().hline(
                    ui.max_rect().x_range().expand(18.0),
                    top,
                    egui::Stroke::new(1.0, Color32::from_gray(0xDC)),
                );
                self.buttons(ui)
            });
        egui::CentralPanel::default_margins()
            .frame(egui::Frame::NONE.fill(PAGE).inner_margin(egui::Margin { left: 22, right: 18, top: 12, bottom: 12 }))
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| step_pill(ui, self.step()));
                ui.add_space(6.0);
                self.page(ui)
            });
    }
}

fn setup_style(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for (name, bytes) in [
        ("body", FONT_BODY),
        ("body_bold", FONT_BODY_BOLD),
        ("narrow", FONT_NARROW),
        ("narrow_bold", FONT_NARROW_BOLD),
    ] {
        let mut data = egui::FontData::from_static(bytes);
        if name.starts_with("body") {
            data = data.tweak(egui::epaint::text::FontTweak {
                subpixel_binning: Some(false),
                hinting: Some(false),
                ..Default::default()
            });
        }
        fonts.font_data.insert(name.into(), std::sync::Arc::new(data));
    }
    let fallback = fonts.families[&egui::FontFamily::Proportional].clone();
    let proportional = fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap();
    proportional.insert(0, "body".into());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, "body".into());
    for name in ["body_bold", "narrow", "narrow_bold"] {
        let mut family = vec![name.to_string()];
        family.extend(fallback.iter().cloned());
        fonts.families.insert(egui::FontFamily::Name(name.into()), family);
    }
    ctx.set_fonts(fonts);
    ctx.set_visuals(egui::Visuals::light());
    ctx.all_styles_mut(|style| {
        use egui::{FontId, TextStyle};
        style.text_styles = [
            (TextStyle::Small, FontId::proportional(16.0)),
            (TextStyle::Body, FontId::proportional(16.0)),
            (TextStyle::Button, FontId::proportional(16.0)),
            (TextStyle::Monospace, FontId::monospace(16.0)),
            (TextStyle::Heading, narrow_bold(22.0)),
        ]
        .into();
        let v = &mut style.visuals;
        v.override_text_color = Some(TEXT);
        v.panel_fill = PAGE;
        v.window_fill = PAGE;
        v.extreme_bg_color = Color32::WHITE;
        v.selection.bg_fill = Color32::from_rgb(0xF2, 0xB8, 0xB8);
        v.selection.stroke = egui::Stroke::new(1.0, RED);
        v.hyperlink_color = LINK;
        v.text_cursor.stroke = egui::Stroke::new(1.0, TEXT);
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, Color32::from_gray(0xA0));
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, BOX_LINE);
        v.widgets.active.bg_stroke = egui::Stroke::new(1.0, RED);
        for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.noninteractive] {
            w.corner_radius = egui::CornerRadius::same(2);
        }
        style.spacing.item_spacing = egui::vec2(6.0, 5.0);
        style.spacing.interact_size.y = 22.0;
    });
}

fn load_png(ctx: &egui::Context, name: &str, bytes: &[u8], options: egui::TextureOptions) -> egui::TextureHandle {
    let icon = eframe::icon_data::from_png_bytes(bytes).expect("bundled image is a valid PNG");
    let image = egui::ColorImage::from_rgba_unmultiplied([icon.width as usize, icon.height as usize], &icon.rgba);
    ctx.load_texture(name, image, options)
}

fn body_bold() -> egui::FontId {
    egui::FontId::new(16.0, egui::FontFamily::Name("body_bold".into()))
}

fn narrow_bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("narrow_bold".into()))
}

fn snap(painter: &egui::Painter, pos: egui::Pos2) -> egui::Pos2 {
    use egui::emath::GuiRounding;
    pos.round_to_pixels(painter.pixels_per_point())
}

fn gradient(painter: &egui::Painter, rect: egui::Rect, top: Color32, bottom: Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(mesh);
}

fn heading(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 22.0), egui::Sense::hover());
        let y = rect.center().y + 1.0;
        for x in [rect.left(), rect.left() + 8.0] {
            ui.painter().add(egui::Shape::convex_polygon(
                vec![egui::pos2(x, y - 5.0), egui::pos2(x + 7.0, y), egui::pos2(x, y + 5.0)],
                RED,
                egui::Stroke::NONE,
            ));
        }
        ui.label(RichText::new(text).font(narrow_bold(22.0)).color(RED));
    });
    ui.add_space(8.0);
}

fn bold(ui: &mut egui::Ui, text: &str, color: Color32) -> egui::Response {
    ui.label(RichText::new(text).font(body_bold()).color(color))
}

fn link(ui: &mut egui::Ui, text: &str, url: &str) -> egui::Response {
    ui.hyperlink_to(RichText::new(text).font(body_bold()).color(LINK).underline(), url)
}

fn bullet(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center() + egui::vec2(0.0, 1.0), 2.5, TEXT);
        ui.label(text);
    });
}

fn pink_box<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::NONE
        .fill(BOX_FILL)
        .stroke(egui::Stroke::new(1.0, BOX_LINE))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn red_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let font = narrow_bold(17.0);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, Color32::WHITE);
    let size = egui::vec2((galley.size().x + 34.0).max(96.0), 30.0);
    let sense = if enabled { egui::Sense::click() } else { egui::Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let (top, bottom, line) = if !enabled {
        (Color32::from_gray(0xC8), Color32::from_gray(0xA8), Color32::from_gray(0x90))
    } else if response.is_pointer_button_down_on() {
        (Color32::from_rgb(0xB0, 0x1C, 0x10), Color32::from_rgb(0xD0, 0x3C, 0x2C), Color32::from_rgb(0x7A, 0x14, 0x08))
    } else if response.hovered() {
        (Color32::from_rgb(0xEE, 0x5E, 0x4C), Color32::from_rgb(0xC6, 0x28, 0x18), Color32::from_rgb(0x8C, 0x1A, 0x0A))
    } else {
        (Color32::from_rgb(0xE0, 0x4C, 0x3C), Color32::from_rgb(0xB4, 0x1C, 0x10), Color32::from_rgb(0x8C, 0x1A, 0x0A))
    };
    let painter = ui.painter();
    gradient(painter, rect, top, bottom);
    painter.hline(rect.x_range().shrink(1.0), rect.top() + 1.0, egui::Stroke::new(1.0, Color32::from_white_alpha(70)));
    painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, line), egui::StrokeKind::Inside);
    let pos = snap(painter, rect.center() - galley.size() / 2.0);
    painter.galley(pos + egui::vec2(1.0, 1.0), galley.clone(), Color32::from_black_alpha(90));
    painter.galley_with_override_text_color(pos, galley, Color32::WHITE);
    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn gray_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let color = if enabled { Color32::from_gray(0x2A) } else { Color32::from_gray(0x9A) };
    let galley = ui.painter().layout_no_wrap(text.to_owned(), narrow_bold(15.0), color);
    let size = egui::vec2((galley.size().x + 26.0).max(74.0), 30.0);
    let sense = if enabled { egui::Sense::click() } else { egui::Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let (top, bottom) = if enabled && response.is_pointer_button_down_on() {
        (Color32::from_gray(0xC8), Color32::from_gray(0xE8))
    } else if enabled && response.hovered() {
        (Color32::from_gray(0xFF), Color32::from_gray(0xDA))
    } else {
        (Color32::from_gray(0xF6), Color32::from_gray(0xD2))
    };
    let painter = ui.painter();
    gradient(painter, rect, top, bottom);
    let line = if enabled && response.hovered() { BOX_LINE } else { Color32::from_gray(0x9A) };
    painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, line), egui::StrokeKind::Inside);
    painter.galley_with_override_text_color(snap(painter, rect.center() - galley.size() / 2.0), galley, color);
    response
}

fn step_pill(ui: &mut egui::Ui, current: usize) {
    let font = body_bold();
    let galleys: Vec<_> = STEPS
        .iter()
        .enumerate()
        .map(|(i, step)| {
            let color = match i.cmp(&current) {
                std::cmp::Ordering::Equal => Color32::WHITE,
                std::cmp::Ordering::Less => Color32::from_gray(0xC8),
                std::cmp::Ordering::Greater => Color32::from_gray(0x8C),
            };
            ui.painter().layout_no_wrap(step.to_string(), font.clone(), color)
        })
        .collect();
    let gap = 18.0;
    let width = galleys.iter().map(|g| g.size().x).sum::<f32>() + gap * (STEPS.len() as f32 - 1.0) + 32.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 12.0, Color32::from_gray(0x4A));
    painter.rect_filled(
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.center().y)).shrink2(egui::vec2(8.0, 1.0)),
        8.0,
        Color32::from_white_alpha(14),
    );
    painter.rect_stroke(rect, 12.0, egui::Stroke::new(1.0, Color32::from_gray(0x8A)), egui::StrokeKind::Inside);
    let mut x = rect.left() + 16.0;
    for galley in galleys {
        let w = galley.size().x;
        let pos = snap(painter, egui::pos2(x, rect.center().y - galley.size().y / 2.0));
        painter.galley(pos, galley, Color32::WHITE);
        x += w + gap;
    }
}

fn field(ui: &mut egui::Ui, name: &str, value: &str) {
    bold(ui, name, TEXT);
    ui.add(egui::Label::new(value).wrap());
    ui.add_space(6.0);
}

fn build_name(build: Build) -> &'static str {
    match build {
        Build::Windows => "Windows",
        Build::Proton => "Windows build through Proton",
        Build::Linux => "Linux",
        Build::MacOS => "macOS",
    }
}

fn warnings(ui: &mut egui::Ui, game: &Game) {
    for warning in game.warnings() {
        ui.add_space(6.0);
        ui.colored_label(AMBER, format!("⚠ {warning}"));
    }
}
