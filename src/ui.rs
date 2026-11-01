use crate::{
    auth::{Backend, Event, Power, Session},
    config::Config,
    policy::{self, Conversation, Phase},
};
use gtk::{gdk, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
use zeroize::Zeroizing;

pub struct Greeter {
    pub window: gtk::ApplicationWindow,
    backend: Backend,
    config: Rc<Config>,
    stack: gtk::Stack,
    username: gtk::Entry,
    password: gtk::PasswordEntry,
    submit: gtk::Button,
    cancel: gtk::Button,
    status: gtk::Label,
    spinner: gtk::Spinner,
    heading_label: gtk::Label,
    domain_hint_label: gtk::Label,
    pub user_tile_preselected: gtk::Button,
    pub user_tile_manual: gtk::Button,
    pub is_manual: Cell<bool>,
    pub default_user: String,
    sessions: RefCell<Vec<Session>>,
    state: RefCell<Conversation>,
    connected: Cell<bool>,
    generation: Cell<u64>,
}

fn label(text: &str, class: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    if !class.is_empty() {
        label.add_css_class(class);
    }
    label
}

fn default_avatar_texture() -> Option<gdk::Texture> {
    let bytes = glib::Bytes::from_static(include_bytes!("../data/avatar.png"));
    gdk::Texture::from_bytes(&bytes).ok()
}

pub fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.connect_parsing_error(|_, section, error| {
        eprintln!("CSS {:?}: {error}", section.start_location());
    });
    provider.load_from_data(include_str!("../style.css"));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

impl Greeter {
    pub fn build(app: &gtk::Application, backend: Backend, config: &Config) -> Rc<Self> {
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("Monolito · Entrar")
            .default_width(1280)
            .default_height(800)
            .build();
        window.add_css_class("greeter");

        let overlay = gtk::Overlay::new();
        if let Some(path) = &config.wallpaper {
            let picture = gtk::Picture::for_filename(path);
            picture.set_content_fit(gtk::ContentFit::Cover);
            overlay.set_child(Some(&picture));
        } else {
            overlay.set_child(Some(&crate::background::landscape()));
        }

        let scrim = gtk::Box::new(gtk::Orientation::Vertical, 0);
        scrim.add_css_class("scrim");
        scrim.set_can_target(false);
        overlay.add_overlay(&scrim);

        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(250)
            .vexpand(true)
            .hexpand(true)
            .build();

        let hostname = glib::host_name();
        let default_user = if !config.default_user.is_empty() {
            config.default_user.clone()
        } else if let Some(hint) = backend.select_user_hint() {
            hint
        } else {
            "rafael".to_string()
        };

        let avatar_texture = default_avatar_texture();

        // ------------------------------------------------------------------
        // LOCK SCREEN (Windows 11 clean hierarchy)
        // ------------------------------------------------------------------
        let lock_root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        lock_root.set_margin_start(40);
        lock_root.set_margin_end(40);
        lock_root.set_margin_top(32);
        lock_root.set_margin_bottom(32);

        // Lock screen top bar: hostname on left, system utilities on right
        let lock_header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let host_lock_label = label(&hostname, "hostname-label");
        lock_header.append(&host_lock_label);

        if backend.is_demo() {
            lock_header.append(&label("PRÉVIA", "demo-badge"));
        }

        let lock_header_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        lock_header_spacer.set_hexpand(true);
        lock_header.append(&lock_header_spacer);

        let lock_net_icon = gtk::Image::from_icon_name("network-wired-symbolic");
        lock_net_icon.add_css_class("utility-icon");
        lock_header.append(&lock_net_icon);

        let access_lock = gtk::MenuButton::builder()
            .icon_name("preferences-desktop-accessibility-symbolic")
            .tooltip_text("Acessibilidade")
            .build();
        access_lock.add_css_class("flat-utility-btn");
        access_lock.update_property(&[gtk::accessible::Property::Label("Acessibilidade")]);
        lock_header.append(&access_lock);

        let power_lock = gtk::MenuButton::builder()
            .icon_name("system-shutdown-symbolic")
            .tooltip_text("Energia")
            .build();
        power_lock.add_css_class("flat-utility-btn");
        power_lock.update_property(&[gtk::accessible::Property::Label("Energia")]);
        lock_header.append(&power_lock);

        lock_root.append(&lock_header);

        // Centered clock & date
        let lock_center = gtk::Box::new(gtk::Orientation::Vertical, 4);
        lock_center.set_valign(gtk::Align::Center);
        lock_center.set_halign(gtk::Align::Center);
        lock_center.set_vexpand(true);

        let lock_time = label("", "lock-clock");
        let lock_date = label("", "lock-date");
        lock_center.append(&lock_time);
        lock_center.append(&lock_date);
        lock_root.append(&lock_center);

        // Lock bottom unlock indicator
        let lock_bottom = gtk::Box::new(gtk::Orientation::Vertical, 0);
        lock_bottom.set_halign(gtk::Align::Center);

        let unlock_button = gtk::Button::new();
        unlock_button.add_css_class("unlock-hint");
        let unlock_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        unlock_box.append(&gtk::Image::from_icon_name("pan-up-symbolic"));
        unlock_box.append(&label("Pressione qualquer tecla para desbloquear", ""));
        unlock_button.set_child(Some(&unlock_box));
        lock_bottom.append(&unlock_button);
        lock_root.append(&lock_bottom);

        stack.add_named(&lock_root, Some("lock"));

        // ------------------------------------------------------------------
        // LOGIN SCREEN (Windows 11 direct layout, NO card container)
        // ------------------------------------------------------------------
        let login_root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        login_root.set_margin_start(40);
        login_root.set_margin_end(40);
        login_root.set_margin_top(32);
        login_root.set_margin_bottom(32);

        // Top bar: hostname
        let login_header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let host_login_label = label(&hostname, "hostname-label");
        login_header.append(&host_login_label);
        if backend.is_demo() {
            login_header.append(&label("PRÉVIA · SEM AUTENTICAÇÃO REAL", "demo-badge"));
        }
        let login_header_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        login_header_spacer.set_hexpand(true);
        login_header.append(&login_header_spacer);
        login_root.append(&login_header);

        // Center container: NO card, direct elements floating on blurred background
        let login_center = gtk::Box::new(gtk::Orientation::Vertical, 0);
        login_center.set_valign(gtk::Align::Center);
        login_center.set_halign(gtk::Align::Center);
        login_center.set_vexpand(true);

        let form_container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        form_container.add_css_class("login-form-container");
        form_container.set_halign(gtk::Align::Center);

        // User Avatar Picture (Uploaded Windows 11 style avatar)
        let avatar_widget = if let Some(texture) = &avatar_texture {
            let pic = gtk::Picture::for_paintable(texture);
            pic.set_can_shrink(true);
            pic.set_content_fit(gtk::ContentFit::Cover);
            pic.set_size_request(96, 96);
            pic.add_css_class("avatar-picture");
            pic.set_halign(gtk::Align::Center);
            pic
        } else {
            let pic = gtk::Picture::new();
            pic.set_size_request(96, 96);
            pic.add_css_class("avatar-picture");
            pic.set_halign(gtk::Align::Center);
            pic
        };
        form_container.append(&avatar_widget);

        // User heading: shows active user's name or "Outro usuário"
        let heading_label = label(&default_user, "account-title");
        heading_label.set_halign(gtk::Align::Center);
        form_container.append(&heading_label);

        // Inputs container
        let fields = gtk::Box::new(gtk::Orientation::Vertical, 8);
        fields.set_halign(gtk::Align::Center);
        fields.set_size_request(280, -1);

        // Username field (only visible in manual "Outro usuário" mode)
        let username = gtk::Entry::builder()
            .placeholder_text("Nome de usuário")
            .max_length(512)
            .build();
        username.update_property(&[gtk::accessible::Property::Label(
            "Usuário local ou de domínio",
        )]);
        username.set_input_purpose(gtk::InputPurpose::FreeForm);
        username.set_input_hints(gtk::InputHints::NO_SPELLCHECK | gtk::InputHints::LOWERCASE);
        username.set_text(&default_user);
        username.set_visible(false); // Initially hidden because default user is preselected!
        fields.append(&username);

        // Password row with discrete Windows 11 submit button
        let password_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let password = gtk::PasswordEntry::builder()
            .placeholder_text("Senha")
            .show_peek_icon(true)
            .hexpand(true)
            .build();
        password.update_property(&[gtk::accessible::Property::Label("Senha")]);
        password_row.append(&password);

        let submit = gtk::Button::builder()
            .icon_name("go-next-symbolic")
            .tooltip_text("Entrar")
            .build();
        submit.add_css_class("submit-btn");
        submit.set_sensitive(false);
        submit.update_property(&[gtk::accessible::Property::Label("Entrar")]);
        password_row.append(&submit);

        fields.append(&password_row);

        // Domain hint label (shown in manual login mode)
        let domain_hint_label = label(&format!("Entrar em: {}", config.domain_hint), "account-subtitle");
        domain_hint_label.set_halign(gtk::Align::Center);
        domain_hint_label.set_visible(false);
        fields.append(&domain_hint_label);

        form_container.append(&fields);

        // Status row
        let status_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        status_row.set_halign(gtk::Align::Center);
        let spinner = gtk::Spinner::new();
        spinner.set_visible(false);
        status_row.append(&spinner);

        let status = label("", "status-text");
        status.set_wrap(true);
        status.set_max_width_chars(38);
        status.set_justify(gtk::Justification::Center);
        status_row.append(&status);
        form_container.append(&status_row);

        // Cancel / Back button
        let cancel = gtk::Button::with_label("Voltar");
        cancel.add_css_class("back-button");
        cancel.set_halign(gtk::Align::Center);
        form_container.append(&cancel);

        login_center.append(&form_container);
        login_root.append(&login_center);

        // Bottom utility bar
        let login_footer = gtk::Box::new(gtk::Orientation::Horizontal, 12);

        // Bottom-left: Windows 11 User Switcher list (Default user & Other user)
        let user_switch_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        user_switch_box.add_css_class("user-switch-container");

        // Item 1: Default / Preselected user
        let user_tile_preselected = gtk::Button::new();
        user_tile_preselected.add_css_class("user-switch-item");
        user_tile_preselected.add_css_class("active");
        let preselected_content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        if let Some(texture) = &avatar_texture {
            let pic = gtk::Picture::for_paintable(texture);
            pic.set_can_shrink(true);
            pic.set_content_fit(gtk::ContentFit::Cover);
            pic.set_size_request(32, 32);
            pic.add_css_class("avatar-small");
            preselected_content.append(&pic);
        }
        preselected_content.append(&label(&default_user, "user-switch-label"));
        user_tile_preselected.set_child(Some(&preselected_content));
        user_switch_box.append(&user_tile_preselected);

        // Item 2: Manual login ("Outro usuário")
        let user_tile_manual = gtk::Button::new();
        user_tile_manual.add_css_class("user-switch-item");
        let manual_content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        if let Some(texture) = &avatar_texture {
            let pic = gtk::Picture::for_paintable(texture);
            pic.set_can_shrink(true);
            pic.set_content_fit(gtk::ContentFit::Cover);
            pic.set_size_request(32, 32);
            pic.add_css_class("avatar-small");
            manual_content.append(&pic);
        }
        manual_content.append(&label("Outro usuário", "user-switch-label"));
        user_tile_manual.set_child(Some(&manual_content));
        user_switch_box.append(&user_tile_manual);

        login_footer.append(&user_switch_box);

        let footer_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        footer_spacer.set_hexpand(true);
        login_footer.append(&footer_spacer);

        // Bottom-right: Network, Accessibility, Power (Clean flat buttons)
        let login_net_icon = gtk::Image::from_icon_name("network-wired-symbolic");
        login_net_icon.add_css_class("utility-icon");
        login_footer.append(&login_net_icon);

        let access_login = gtk::MenuButton::builder()
            .icon_name("preferences-desktop-accessibility-symbolic")
            .tooltip_text("Acessibilidade")
            .build();
        access_login.add_css_class("flat-utility-btn");
        access_login.update_property(&[gtk::accessible::Property::Label("Acessibilidade")]);
        login_footer.append(&access_login);

        let power_login = gtk::MenuButton::builder()
            .icon_name("system-shutdown-symbolic")
            .tooltip_text("Energia")
            .build();
        power_login.add_css_class("flat-utility-btn");
        power_login.update_property(&[gtk::accessible::Property::Label("Energia")]);
        login_footer.append(&power_login);

        login_root.append(&login_footer);
        stack.add_named(&login_root, Some("login"));

        overlay.add_overlay(&stack);
        window.set_child(Some(&overlay));

        let ui = Rc::new(Self {
            window,
            backend,
            config: Rc::new(Config {
                default_user: config.default_user.clone(),
                domain_hint: config.domain_hint.clone(),
                default_session: config.default_session.clone(),
                wallpaper: config.wallpaper.clone(),
                auto_ack: config.auto_ack,
            }),
            stack,
            username,
            password,
            submit,
            cancel,
            status,
            spinner,
            heading_label,
            domain_hint_label,
            user_tile_preselected,
            user_tile_manual,
            is_manual: Cell::new(false),
            default_user,
            sessions: RefCell::new(Vec::new()),
            state: RefCell::new(Conversation::new(config.auto_ack)),
            connected: Cell::new(false),
            generation: Cell::new(0),
        });

        // Set up accessibility & power popovers on both screens
        ui.setup_accessibility(&access_lock);
        ui.setup_accessibility(&access_login);
        ui.setup_power(&power_lock);
        ui.setup_power(&power_login);

        // Switcher bindings
        let weak = Rc::downgrade(&ui);
        ui.user_tile_preselected.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.set_mode_manual(false);
            }
        });

        let weak = Rc::downgrade(&ui);
        ui.user_tile_manual.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.set_mode_manual(true);
            }
        });

        // Interactivity bindings
        let weak = Rc::downgrade(&ui);
        unlock_button.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.reveal();
            }
        });

        let weak = Rc::downgrade(&ui);
        ui.submit.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.submit();
            }
        });

        let weak = Rc::downgrade(&ui);
        ui.username.connect_activate(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.password.grab_focus();
            }
        });

        let weak = Rc::downgrade(&ui);
        ui.password.connect_activate(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.submit();
            }
        });

        let weak = Rc::downgrade(&ui);
        ui.cancel.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.back();
            }
        });

        // Global keyboard controller
        let key = gtk::EventControllerKey::new();
        key.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(&ui);
        key.connect_key_pressed(move |_, key, _, _| {
            if let Some(ui) = weak.upgrade() {
                if ui.stack.visible_child_name().as_deref() == Some("lock") {
                    if !matches!(
                        key,
                        gdk::Key::Shift_L
                            | gdk::Key::Shift_R
                            | gdk::Key::Control_L
                            | gdk::Key::Alt_L
                            | gdk::Key::Super_L
                    ) {
                        ui.reveal();
                        return glib::Propagation::Stop;
                    }
                } else if key == gdk::Key::Escape {
                    ui.back();
                    return glib::Propagation::Stop;
                }
            }
            glib::Propagation::Proceed
        });
        ui.window.add_controller(key);

        // Lock screen gesture controllers
        let click = gtk::GestureClick::new();
        let weak = Rc::downgrade(&ui);
        click.connect_released(move |_, _, _, _| {
            if let Some(ui) = weak.upgrade() {
                ui.reveal();
            }
        });
        lock_root.add_controller(click);

        let drag = gtk::GestureDrag::new();
        let weak = Rc::downgrade(&ui);
        drag.connect_drag_end(move |_, _, y| {
            if y.abs() > 30. {
                if let Some(ui) = weak.upgrade() {
                    ui.reveal();
                }
            }
        });
        lock_root.add_controller(drag);

        // Clock timer
        let tick = move || {
            if let Ok(now) = glib::DateTime::now_local() {
                if let Ok(time) = now.format("%H:%M") {
                    lock_time.set_text(&time);
                }
                let days = [
                    "segunda-feira",
                    "terça-feira",
                    "quarta-feira",
                    "quinta-feira",
                    "sexta-feira",
                    "sábado",
                    "domingo",
                ];
                let months = [
                    "janeiro",
                    "fevereiro",
                    "março",
                    "abril",
                    "maio",
                    "junho",
                    "julho",
                    "agosto",
                    "setembro",
                    "outubro",
                    "novembro",
                    "dezembro",
                ];
                lock_date.set_text(&format!(
                    "{}, {} de {}",
                    days[(now.day_of_week() - 1) as usize],
                    now.day_of_month(),
                    months[(now.month() - 1) as usize]
                ));
            }
        };
        tick();
        glib::timeout_add_local(Duration::from_secs(1), move || {
            tick();
            glib::ControlFlow::Continue
        });

        ui
    }

    pub fn set_mode_manual(&self, manual: bool) {
        self.is_manual.set(manual);
        self.clear_secrets();
        self.status("", false);
        if manual {
            self.heading_label.set_text("Outro usuário");
            self.username.set_visible(true);
            self.username.set_text("");
            self.domain_hint_label.set_visible(true);
            self.user_tile_preselected.remove_css_class("active");
            self.user_tile_manual.add_css_class("active");
            self.username.grab_focus();
        } else {
            self.heading_label.set_text(&self.default_user);
            self.username.set_visible(false);
            self.username.set_text(&self.default_user);
            self.domain_hint_label.set_visible(false);
            self.user_tile_preselected.add_css_class("active");
            self.user_tile_manual.remove_css_class("active");
            self.password.grab_focus();
        }
    }

    fn reveal(&self) {
        self.stack.set_visible_child_name("login");
        self.set_mode_manual(false);
    }

    pub fn clear_secrets(&self) {
        self.password.set_text("");
        self.state.borrow_mut().clear();
    }

    fn status(&self, text: &str, error: bool) {
        self.status.set_text(text);
        if error {
            self.status.add_css_class("error");
        } else {
            self.status.remove_css_class("error");
        }
    }

    fn busy(&self, busy: bool) {
        if busy {
            self.cancel.grab_focus();
        }
        self.spinner.set_visible(busy);
        self.spinner.set_spinning(busy);
        self.username.set_sensitive(!busy);
        self.password.set_sensitive(!busy);
        self.submit.set_sensitive(!busy && self.connected.get());
        self.user_tile_preselected.set_sensitive(!busy);
        self.user_tile_manual.set_sensitive(!busy);
    }

    pub fn connect(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.backend.subscribe(Rc::new(move |event| {
            if let Some(ui) = weak.upgrade() {
                ui.event(event);
            }
        }));
        let weak = Rc::downgrade(self);
        self.backend.connect(move |result| {
            if let Some(ui) = weak.upgrade() {
                match result {
                    Ok(()) => {
                        let sessions = ui.backend.sessions();
                        let has_sessions = !sessions.is_empty();
                        *ui.sessions.borrow_mut() = sessions;
                        ui.connected.set(has_sessions);
                        ui.busy(false);
                        if !has_sessions {
                            ui.status("Nenhuma sessão instalada no sistema.", true);
                        } else if ui.backend.is_demo() {
                            ui.status("Modo prévia: senha «monolito»", false);
                        } else {
                            ui.status("", false);
                        }
                    }
                    Err(_) => {
                        ui.status("LightDM indisponível. Inicie pelo gerenciador de login ou use --demo.", true);
                        ui.busy(false);
                    }
                }
            }
        });
    }

    fn submit(self: &Rc<Self>) {
        if !self.connected.get() {
            return;
        }
        let phase = self.state.borrow().phase;
        match phase {
            Phase::Idle => {
                let user_str = if self.is_manual.get() {
                    self.username.text().to_string()
                } else {
                    self.default_user.clone()
                };
                let user = user_str.trim();
                if !policy::valid_username(user) {
                    self.status("Informe seu usuário local ou de domínio.", true);
                    if self.is_manual.get() {
                        self.username.grab_focus();
                    }
                    return;
                }
                let password = Zeroizing::new(self.password.text().to_string());
                self.password.set_text("");
                self.state.borrow_mut().begin(password.to_string());
                self.busy(true);
                self.status("Verificando credenciais…", false);
                self.cancel.set_label("Cancelar");
                if self.backend.authenticate(user).is_err() {
                    self.failed("Não foi possível iniciar a autenticação.");
                    return;
                }
                let generation = self.generation.get() + 1;
                self.generation.set(generation);
                let weak = Rc::downgrade(self);
                glib::timeout_add_local_once(Duration::from_secs(90), move || {
                    if let Some(ui) = weak.upgrade() {
                        if ui.generation.get() == generation
                            && matches!(
                                ui.state.borrow().phase,
                                Phase::Authenticating | Phase::Prompt { .. }
                            )
                        {
                            ui.cancel_attempt("Tempo de autenticação esgotado.");
                        }
                    }
                });
            }
            _ => {}
        }
    }

    fn event(self: &Rc<Self>, event: Event) {
        match event {
            Event::Prompt(text, secret) => {
                if !matches!(
                    self.state.borrow().phase,
                    Phase::Authenticating | Phase::Prompt { .. }
                ) {
                    return;
                }
                let response = self.state.borrow_mut().prompt(&text, secret);
                if let Some(response) = response {
                    if self.backend.respond(&response).is_err() {
                        self.cancel_attempt("Falha na comunicação com o LightDM.");
                    }
                } else {
                    self.cancel_attempt("Desafio de autenticação adicional não suportado.");
                }
            }
            Event::Message(text, error) => {
                if matches!(
                    self.state.borrow().phase,
                    Phase::Authenticating | Phase::Prompt { .. }
                ) {
                    let warning = text.to_lowercase();
                    if error || !(warning.contains("arcfour") && warning.contains("deprecated")) {
                        self.status(&text, error);
                    }
                }
            }
            Event::Complete(authenticated) => {
                if self.state.borrow().phase == Phase::Cancelling {
                    self.failed("Autenticação cancelada.");
                    return;
                }
                if !matches!(
                    self.state.borrow().phase,
                    Phase::Authenticating | Phase::Prompt { .. }
                ) {
                    return;
                }
                self.generation.set(self.generation.get() + 1);
                self.clear_secrets();
                if authenticated {
                    self.start_session();
                } else {
                    self.failed("Nome de usuário ou senha incorretos.");
                }
            }
            Event::Reset => {
                self.clear_secrets();
                self.username.set_text("");
                self.busy(false);
                self.stack.set_visible_child_name("lock");
            }
        }
    }

    fn start_session(self: &Rc<Self>) {
        // Preferred session from config (default 'xfce'), fallback to backend default or first installed
        let sessions = self.sessions.borrow();
        let target_key = if let Some(s) = sessions.iter().find(|s| s.key == self.config.default_session) {
            s.key.clone()
        } else {
            let default_hint = self.backend.default_session();
            if let Some(s) = sessions.iter().find(|s| s.key == default_hint) {
                s.key.clone()
            } else if let Some(s) = sessions.first() {
                s.key.clone()
            } else {
                self.config.default_session.clone()
            }
        };
        drop(sessions);

        self.state.borrow_mut().phase = Phase::Session;
        self.busy(true);
        self.cancel.set_sensitive(false);
        self.status("Iniciando sessão…", false);
        let weak = Rc::downgrade(self);
        self.backend.start_session(&target_key, move |result| {
            if let Some(ui) = weak.upgrade() {
                match result {
                    Ok(()) => {
                        ui.state.borrow_mut().phase = Phase::Complete;
                        ui.spinner.set_spinning(false);
                        ui.spinner.set_visible(false);
                        ui.status(
                            if ui.backend.is_demo() {
                                "Prévia concluída. Nenhuma sessão real foi aberta."
                            } else {
                                "Sessão iniciada."
                            },
                            false,
                        );
                        if ui.backend.is_demo() {
                            ui.cancel.set_sensitive(true);
                            ui.cancel.set_label("Voltar");
                        }
                    }
                    Err(_) => ui.failed("Falha ao abrir a sessão do sistema."),
                }
            }
        });
    }

    fn failed(&self, message: &str) {
        self.clear_secrets();
        self.busy(false);
        self.cancel.set_sensitive(true);
        self.cancel.set_label("Voltar");
        self.status(message, true);
        self.password.grab_focus();
    }

    fn cancel_attempt(&self, message: &str) {
        self.state.borrow_mut().cancel();
        self.password.set_text("");
        self.busy(true);
        self.status(message, true);
        if self.backend.cancel().is_err() {
            self.connected.set(false);
            self.busy(false);
            self.status(
                "LightDM não confirmou o cancelamento. Reinicie o greeter antes de tentar novamente.",
                true,
            );
        }
    }

    fn back(&self) {
        let phase = self.state.borrow().phase;
        match phase {
            Phase::Session | Phase::Cancelling => return,
            Phase::Authenticating | Phase::Prompt { .. } => {}
            _ => {
                self.clear_secrets();
                self.set_mode_manual(false);
                self.busy(false);
                self.stack.set_visible_child_name("lock");
                return;
            }
        }
        self.cancel_attempt("Cancelando autenticação…");
    }

    fn setup_accessibility(&self, button: &gtk::MenuButton) {
        let popover = gtk::Popover::new();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
        content.set_margin_top(10);
        content.set_margin_bottom(10);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.append(&label("Acessibilidade", "popover-header"));

        for (title, class) in [
            ("Alto contraste", "high-contrast"),
            ("Texto maior", "large-text"),
            ("Reduzir movimento", "reduced-motion"),
        ] {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 24);
            let name = label(title, "");
            name.set_hexpand(true);
            name.set_xalign(0.);
            row.append(&name);

            let switch = gtk::Switch::new();
            switch.update_property(&[gtk::accessible::Property::Label(title)]);
            let window = self.window.downgrade();
            let stack = self.stack.downgrade();
            switch.connect_active_notify(move |switch| {
                if let Some(window) = window.upgrade() {
                    if switch.is_active() {
                        window.add_css_class(class);
                    } else {
                        window.remove_css_class(class);
                    }
                }
                if class == "reduced-motion" {
                    if let Some(stack) = stack.upgrade() {
                        stack.set_transition_duration(if switch.is_active() { 0 } else { 250 });
                    }
                }
            });
            row.append(&switch);
            content.append(&row);
        }
        popover.set_child(Some(&content));
        button.set_popover(Some(&popover));
    }

    fn setup_power(self: &Rc<Self>, button: &gtk::MenuButton) {
        let popover = gtk::Popover::new();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 4);
        content.set_margin_top(6);
        content.set_margin_bottom(6);
        content.set_margin_start(6);
        content.set_margin_end(6);

        for power in [Power::Suspend, Power::Restart, Power::Shutdown] {
            let action = gtk::Button::with_label(power.label());
            action.add_css_class("power-item");
            action.set_sensitive(self.backend.can_power(power));
            let weak = Rc::downgrade(self);
            let popup = popover.downgrade();
            action.connect_clicked(move |_| {
                if let Some(popup) = popup.upgrade() {
                    popup.popdown();
                }
                if let Some(ui) = weak.upgrade() {
                    let dialog = gtk::MessageDialog::builder()
                        .transient_for(&ui.window)
                        .modal(true)
                        .message_type(gtk::MessageType::Question)
                        .buttons(gtk::ButtonsType::YesNo)
                        .text(format!("{} o computador?", power.label()))
                        .secondary_text(if ui.backend.is_demo() {
                            "Modo prévia: nenhuma ação será executada."
                        } else {
                            "Esta ação será executada pelo serviço de energia do sistema."
                        })
                        .build();
                    let weak = Rc::downgrade(&ui);
                    dialog.connect_response(move |dialog, response| {
                        if response == gtk::ResponseType::Yes {
                            if let Some(ui) = weak.upgrade() {
                                if ui.backend.power(power).is_err() {
                                    ui.status("O sistema não autorizou a ação de energia.", true);
                                }
                            }
                        }
                        dialog.close();
                    });
                    dialog.present();
                }
            });
            content.append(&action);
        }
        popover.set_child(Some(&content));
        button.set_popover(Some(&popover));
    }
}

/// Drives the actual GTK widgets and demo signal adapter.
/// Single-step direct authentication with preselected user and manual domain login.
pub fn smoke_test(ui: Rc<Greeter>, failed: Rc<Cell<bool>>) {
    let step = Rc::new(Cell::new(0));
    let ticks = Rc::new(Cell::new(0));
    glib::timeout_add_local(Duration::from_millis(100), move || {
        ticks.set(ticks.get() + 1);
        let phase = ui.state.borrow().phase;
        let result = match step.get() {
            0 if ui.connected.get() => {
                ui.reveal();
                // Test preselected user with wrong password
                ui.password.set_text("wrong");
                ui.submit.emit_clicked();
                step.set(1);
                None
            }
            1 if phase == Phase::Idle => {
                if !ui.status.has_css_class("error") {
                    Some(false)
                } else {
                    // Correct password for preselected user
                    ui.password.set_text("monolito");
                    ui.submit.emit_clicked();
                    step.set(2);
                    None
                }
            }
            2 if phase == Phase::Complete => {
                if !ui.password.text().is_empty() {
                    Some(false)
                } else {
                    // Back to lock screen, reveal, then switch to manual login
                    ui.back();
                    ui.reveal();
                    ui.set_mode_manual(true);
                    ui.username.set_text("teste@acme.example");
                    ui.password.set_text("monolito");
                    ui.submit.emit_clicked();
                    step.set(3);
                    None
                }
            }
            3 if phase == Phase::Complete => {
                ui.back();
                step.set(4);
                None
            }
            4 if phase == Phase::Idle => {
                let cleared = ui.password.text().is_empty();
                ui.back();
                Some(
                    cleared
                        && ui.stack.visible_child_name().as_deref() == Some("lock")
                        && ui.backend.is_demo(),
                )
            }
            _ if ticks.get() > 100 => Some(false),
            _ => None,
        };
        if let Some(passed) = result {
            println!(
                "GTK smoke: {} (usuário prévio, senha incorreta, nova tentativa, login manual de domínio, sessão xfce, limpeza)",
                if passed { "PASS" } else { "FAIL" }
            );
            failed.set(!passed);
            if let Some(app) = ui.window.application() {
                app.quit();
            }
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}
