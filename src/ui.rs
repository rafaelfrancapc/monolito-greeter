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
    stack: gtk::Stack,
    username: gtk::Entry,
    password: gtk::PasswordEntry,
    submit: gtk::Button,
    cancel: gtk::Button,
    status: gtk::Label,
    spinner: gtk::Spinner,
    session: gtk::DropDown,
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

        // ------------------------------------------------------------------
        // LOCK SCREEN (Windows 11 / macOS clean hierarchy)
        // ------------------------------------------------------------------
        let lock_root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        lock_root.set_margin_start(40);
        lock_root.set_margin_end(40);
        lock_root.set_margin_top(32);
        lock_root.set_margin_bottom(32);

        // Lock screen top bar
        let lock_header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let brand_label = label("monolito", "brand-wordmark");
        lock_header.append(&brand_label);

        if backend.is_demo() {
            lock_header.append(&label("PRÉVIA", "demo-badge"));
        }

        let lock_header_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        lock_header_spacer.set_hexpand(true);
        lock_header.append(&lock_header_spacer);

        let lock_net_icon = gtk::Image::from_icon_name("network-wired-symbolic");
        lock_net_icon.add_css_class("network-indicator");
        lock_header.append(&lock_net_icon);

        let access_lock = gtk::MenuButton::builder()
            .icon_name("preferences-desktop-accessibility-symbolic")
            .tooltip_text("Acessibilidade")
            .build();
        access_lock.add_css_class("glass-circle-button");
        access_lock.update_property(&[gtk::accessible::Property::Label("Acessibilidade")]);
        lock_header.append(&access_lock);

        let power_lock = gtk::MenuButton::builder()
            .icon_name("system-shutdown-symbolic")
            .tooltip_text("Energia")
            .build();
        power_lock.add_css_class("glass-circle-button");
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
        // LOGIN SCREEN (Windows 11 domain login + macOS liquid glass)
        // ------------------------------------------------------------------
        let login_root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        login_root.set_margin_start(40);
        login_root.set_margin_end(40);
        login_root.set_margin_top(32);
        login_root.set_margin_bottom(32);

        // Top bar
        let login_header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let brand_login = label("monolito", "brand-wordmark");
        login_header.append(&brand_login);
        if backend.is_demo() {
            login_header.append(&label("PRÉVIA · SEM AUTENTICAÇÃO REAL", "demo-badge"));
        }
        let login_header_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        login_header_spacer.set_hexpand(true);
        login_header.append(&login_header_spacer);
        login_root.append(&login_header);

        // Center card container
        let login_center = gtk::Box::new(gtk::Orientation::Vertical, 0);
        login_center.set_valign(gtk::Align::Center);
        login_center.set_halign(gtk::Align::Center);
        login_center.set_vexpand(true);

        let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
        card.add_css_class("login-card");
        card.set_halign(gtk::Align::Center);

        // Large circular avatar (Windows 11 style)
        let avatar_circle = gtk::Box::new(gtk::Orientation::Vertical, 0);
        avatar_circle.add_css_class("avatar-circle");
        avatar_circle.set_halign(gtk::Align::Center);
        avatar_circle.set_valign(gtk::Align::Center);
        let avatar_img = gtk::Image::from_icon_name("avatar-default-symbolic");
        avatar_img.set_pixel_size(44);
        avatar_img.set_halign(gtk::Align::Center);
        avatar_img.set_valign(gtk::Align::Center);
        avatar_circle.append(&avatar_img);
        card.append(&avatar_circle);

        // Account title
        let account_title = label(&config.title, "account-title");
        account_title.set_halign(gtk::Align::Center);
        card.append(&account_title);

        // Domain hint
        let domain_hint = label(&config.domain_hint, "account-subtitle");
        domain_hint.set_halign(gtk::Align::Center);
        card.append(&domain_hint);

        // Fields
        let fields = gtk::Box::new(gtk::Orientation::Vertical, 8);
        fields.set_margin_top(4);

        let username = gtk::Entry::builder()
            .placeholder_text("Nome de usuário ou DOMÍNIO\\usuário")
            .max_length(512)
            .build();
        username.update_property(&[gtk::accessible::Property::Label(
            "Usuário local ou de domínio",
        )]);
        username.set_input_purpose(gtk::InputPurpose::FreeForm);
        username.set_input_hints(gtk::InputHints::NO_SPELLCHECK | gtk::InputHints::LOWERCASE);
        fields.append(&username);

        // Password row with inline submit arrow button
        let password_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
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
        submit.add_css_class("submit-arrow-btn");
        submit.set_sensitive(false);
        submit.update_property(&[gtk::accessible::Property::Label("Entrar")]);
        password_row.append(&submit);

        fields.append(&password_row);
        card.append(&fields);

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
        card.append(&status_row);

        // Cancel / Back button
        let cancel = gtk::Button::with_label("Voltar");
        cancel.add_css_class("back-button");
        cancel.set_halign(gtk::Align::Center);
        card.append(&cancel);

        login_center.append(&card);
        login_root.append(&login_center);

        // Bottom utility bar
        let login_footer = gtk::Box::new(gtk::Orientation::Horizontal, 12);

        // Bottom-left user tile (Windows 11 user switcher)
        let user_switch_pill = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        user_switch_pill.add_css_class("user-switch-pill");
        let switch_avatar = gtk::Image::from_icon_name("avatar-default-symbolic");
        switch_avatar.set_pixel_size(18);
        switch_avatar.add_css_class("user-switch-avatar");
        user_switch_pill.append(&switch_avatar);
        user_switch_pill.append(&label(&config.title, "user-switch-label"));
        login_footer.append(&user_switch_pill);

        let footer_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        footer_spacer.set_hexpand(true);
        login_footer.append(&footer_spacer);

        // Bottom-right controls: Session, Access, Power
        let session_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        session_box.add_css_class("session-pill");
        session_box.append(&gtk::Image::from_icon_name("video-display-symbolic"));
        let session = gtk::DropDown::from_strings(&["Carregando sessões…"]);
        session.set_sensitive(false);
        session.update_property(&[gtk::accessible::Property::Label("Sessão do sistema")]);
        session_box.append(&session);
        login_footer.append(&session_box);

        let access_login = gtk::MenuButton::builder()
            .icon_name("preferences-desktop-accessibility-symbolic")
            .tooltip_text("Acessibilidade")
            .build();
        access_login.add_css_class("glass-circle-button");
        access_login.update_property(&[gtk::accessible::Property::Label("Acessibilidade")]);
        login_footer.append(&access_login);

        let power_login = gtk::MenuButton::builder()
            .icon_name("system-shutdown-symbolic")
            .tooltip_text("Energia")
            .build();
        power_login.add_css_class("glass-circle-button");
        power_login.update_property(&[gtk::accessible::Property::Label("Energia")]);
        login_footer.append(&power_login);

        login_root.append(&login_footer);
        stack.add_named(&login_root, Some("login"));

        overlay.add_overlay(&stack);
        window.set_child(Some(&overlay));

        let ui = Rc::new(Self {
            window,
            backend,
            stack,
            username,
            password,
            submit,
            cancel,
            status,
            spinner,
            session,
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

    fn reveal(&self) {
        self.stack.set_visible_child_name("login");
        self.username.grab_focus();
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
        self.session
            .set_sensitive(!busy && !self.sessions.borrow().is_empty());
        self.submit.set_sensitive(!busy && self.connected.get());
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
                        let names: Vec<_> = sessions.iter().map(|s| s.name.as_str()).collect();
                        ui.session.set_model(Some(&gtk::StringList::new(&names)));
                        let default = ui.backend.default_session();
                        if let Some(index) = sessions.iter().position(|s| s.key == default) {
                            ui.session.set_selected(index as u32);
                        }
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
                let user = self.username.text();
                let user = user.trim();
                if !policy::valid_username(user) {
                    self.status("Informe seu usuário local ou de domínio.", true);
                    self.username.grab_focus();
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
                    // Direct password authentication only. Unsupported prompts cancel cleanly.
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
        let key = self
            .sessions
            .borrow()
            .get(self.session.selected() as usize)
            .map(|s| s.key.clone());
        let Some(key) = key else {
            self.failed("Selecione uma sessão válida.");
            return;
        };
        self.state.borrow_mut().phase = Phase::Session;
        self.busy(true);
        self.cancel.set_sensitive(false);
        self.status("Iniciando sessão…", false);
        let weak = Rc::downgrade(self);
        self.backend.start_session(&key, move |result| {
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
                self.username.set_text("");
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
/// Single-step direct authentication for local accounts and Active Directory.
pub fn smoke_test(ui: Rc<Greeter>, failed: Rc<Cell<bool>>) {
    let step = Rc::new(Cell::new(0));
    let ticks = Rc::new(Cell::new(0));
    glib::timeout_add_local(Duration::from_millis(100), move || {
        ticks.set(ticks.get() + 1);
        let phase = ui.state.borrow().phase;
        let result = match step.get() {
            0 if ui.connected.get() => {
                ui.reveal();
                ui.username.set_text("ACME\\teste");
                ui.password.set_text("wrong");
                ui.submit.emit_clicked();
                step.set(1);
                None
            }
            1 if phase == Phase::Idle => {
                if !ui.status.has_css_class("error") {
                    Some(false)
                } else {
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
                    ui.back();
                    ui.reveal();
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
                        && ui.username.text().is_empty()
                        && ui.stack.visible_child_name().as_deref() == Some("lock")
                        && ui.backend.is_demo(),
                )
            }
            _ if ticks.get() > 100 => Some(false),
            _ => None,
        };
        if let Some(passed) = result {
            println!(
                "GTK smoke: {} (senha inválida, nova tentativa AD, aviso ARCFOUR, senha direta, sessão simulada, cancelamento, limpeza de campos)",
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
