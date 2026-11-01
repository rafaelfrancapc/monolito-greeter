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
    challenge: gtk::Entry,
    challenge_label: gtk::Label,
    challenge_box: gtk::Box,
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

        let layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        layout.set_margin_start(36);
        layout.set_margin_end(36);
        layout.set_margin_top(26);
        layout.set_margin_bottom(26);
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let brand = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        brand.append(&label("M", "monogram"));
        brand.append(&label("monolito", "wordmark"));
        brand.set_hexpand(true);
        header.append(&brand);
        if backend.is_demo() {
            header.append(&label("PRÉVIA · SEM AUTENTICAÇÃO REAL", "demo-badge"));
        }
        let access = gtk::MenuButton::builder()
            .icon_name("preferences-desktop-accessibility-symbolic")
            .tooltip_text("Acessibilidade")
            .build();
        access.add_css_class("round-button");
        access.update_property(&[gtk::accessible::Property::Label("Acessibilidade")]);
        let power = gtk::MenuButton::builder()
            .icon_name("system-shutdown-symbolic")
            .tooltip_text("Energia")
            .build();
        power.add_css_class("round-button");
        power.update_property(&[gtk::accessible::Property::Label("Energia")]);
        header.append(&access);
        header.append(&power);
        layout.append(&header);

        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(280)
            .vexpand(true)
            .hexpand(true)
            .build();
        let lock = gtk::Box::new(gtk::Orientation::Vertical, 4);
        lock.set_valign(gtk::Align::Fill);
        lock.set_halign(gtk::Align::Center);
        let lock_time = label("", "clock");
        lock_time.set_margin_top(55);
        let lock_date = label("", "date");
        lock.append(&lock_time);
        lock.append(&lock_date);
        let spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
        spacer.set_vexpand(true);
        lock.append(&spacer);
        let welcome = gtk::Button::new();
        welcome.add_css_class("welcome-pill");
        let welcome_content = gtk::Box::new(gtk::Orientation::Horizontal, 14);
        welcome_content.append(&label("M", "mini-avatar"));
        let welcome_text = gtk::Box::new(gtk::Orientation::Vertical, 3);
        let title = label("Seu espaço. Seu próximo passo.", "welcome-title");
        title.set_xalign(0.);
        welcome_text.append(&title);
        let subtitle = label("Clique ou pressione uma tecla para entrar", "muted");
        subtitle.set_xalign(0.);
        welcome_text.append(&subtitle);
        welcome_content.append(&welcome_text);
        welcome_content.append(&gtk::Image::from_icon_name("go-next-symbolic"));
        welcome.set_child(Some(&welcome_content));
        lock.append(&welcome);
        lock.set_margin_bottom(52);
        stack.add_named(&lock, Some("lock"));

        let login = gtk::Box::new(gtk::Orientation::Vertical, 12);
        login.set_halign(gtk::Align::Center);
        login.set_valign(gtk::Align::Center);
        let small_clock = label("", "small-clock");
        login.append(&small_clock);
        let card = gtk::Box::new(gtk::Orientation::Vertical, 14);
        card.add_css_class("login-card");
        card.set_size_request(388, -1);
        let avatar = gtk::Image::from_icon_name("avatar-default-symbolic");
        avatar.set_pixel_size(34);
        avatar.add_css_class("avatar");
        avatar.set_halign(gtk::Align::Center);
        card.append(&avatar);
        card.append(&label(&config.title, "title"));
        card.append(&label(&config.domain_hint, "subtitle"));
        let fields = gtk::Box::new(gtk::Orientation::Vertical, 10);
        fields.set_margin_top(10);
        let user_label = label("Usuário", "field-label");
        user_label.set_xalign(0.);
        fields.append(&user_label);
        let username = gtk::Entry::builder()
            .placeholder_text("usuario@dominio ou DOMINIO\\usuario")
            .max_length(512)
            .build();
        username.update_property(&[gtk::accessible::Property::Label(
            "Usuário local ou de domínio",
        )]);
        username.set_input_purpose(gtk::InputPurpose::FreeForm);
        username.set_input_hints(gtk::InputHints::NO_SPELLCHECK | gtk::InputHints::LOWERCASE);
        fields.append(&username);
        let password_label = label("Senha", "field-label");
        password_label.set_xalign(0.);
        fields.append(&password_label);
        let password = gtk::PasswordEntry::builder()
            .placeholder_text("Digite sua senha")
            .show_peek_icon(true)
            .build();
        password.update_property(&[gtk::accessible::Property::Label("Senha")]);
        fields.append(&password);
        let challenge_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let challenge_label = label("", "field-label");
        challenge_label.set_wrap(true);
        challenge_label.set_max_width_chars(38);
        challenge_label.set_xalign(0.);
        let challenge = gtk::Entry::new();
        challenge.set_visibility(false);
        challenge.set_input_hints(gtk::InputHints::PRIVATE | gtk::InputHints::NO_SPELLCHECK);
        challenge.update_property(&[gtk::accessible::Property::Label(
            "Resposta ao desafio de autenticação",
        )]);
        challenge_box.append(&challenge_label);
        challenge_box.append(&challenge);
        challenge_box.set_visible(false);
        fields.append(&challenge_box);
        card.append(&fields);
        let submit = gtk::Button::with_label("Entrar  →");
        submit.add_css_class("primary");
        submit.set_sensitive(false);
        card.append(&submit);
        let status_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        status_row.set_halign(gtk::Align::Center);
        let spinner = gtk::Spinner::new();
        spinner.set_visible(false);
        status_row.append(&spinner);
        let status = label("Conectando ao serviço de login…", "status");
        status.set_wrap(true);
        status.set_max_width_chars(40);
        status.set_justify(gtk::Justification::Center);
        status_row.append(&status);
        card.append(&status_row);
        let cancel = gtk::Button::with_label("Voltar");
        cancel.add_css_class("text-button");
        card.append(&cancel);
        login.append(&card);
        let local_note = label("Uma entrada tranquila para o que vem a seguir.", "caption");
        login.append(&local_note);
        // Keep the form reachable on short displays and with enlarged text,
        // including PAM conversations that add another challenge field.
        login.set_margin_top(12);
        login.set_margin_bottom(12);
        login.set_vexpand(true);
        let login_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .propagate_natural_width(true)
            .child(&login)
            .build();
        stack.add_named(&login_scroll, Some("login"));
        layout.append(&stack);

        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let session_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        session_box.add_css_class("session-pill");
        session_box.append(&gtk::Image::from_icon_name("video-display-symbolic"));
        let session = gtk::DropDown::from_strings(&["Carregando sessões…"]);
        session.set_sensitive(false);
        session.update_property(&[gtk::accessible::Property::Label("Sessão do sistema")]);
        session_box.append(&session);
        footer.append(&session_box);
        let footer_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        footer_spacer.set_hexpand(true);
        footer.append(&footer_spacer);
        footer.append(&label("MONOLITO  /  GREETER", "footer-brand"));
        layout.append(&footer);
        overlay.add_overlay(&layout);
        window.set_child(Some(&overlay));

        let ui = Rc::new(Self {
            window,
            backend,
            stack,
            username,
            password,
            challenge,
            challenge_label,
            challenge_box,
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
        ui.setup_accessibility(&access);
        ui.setup_power(&power);
        let weak = Rc::downgrade(&ui);
        welcome.connect_clicked(move |_| {
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
        ui.challenge.connect_activate(move |_| {
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
        let click = gtk::GestureClick::new();
        let weak = Rc::downgrade(&ui);
        click.connect_released(move |_, _, _, _| {
            if let Some(ui) = weak.upgrade() {
                ui.reveal();
            }
        });
        lock.add_controller(click);
        let drag = gtk::GestureDrag::new();
        let weak = Rc::downgrade(&ui);
        drag.connect_drag_end(move |_, _, y| {
            if y.abs() > 35. {
                if let Some(ui) = weak.upgrade() {
                    ui.reveal();
                }
            }
        });
        lock.add_controller(drag);
        let tick = move || {
            if let Ok(now) = glib::DateTime::now_local() {
                if let Ok(time) = now.format("%H:%M") {
                    lock_time.set_text(&time);
                    small_clock.set_text(&time);
                }
                // Portuguese names do not depend on a locale being installed.
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
        self.challenge.set_text("");
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
                        if let Some(index) = sessions.iter().position(|s| s.key == default) { ui.session.set_selected(index as u32); }
                        let has_sessions = !sessions.is_empty();
                        *ui.sessions.borrow_mut() = sessions;
                        ui.connected.set(has_sessions);
                        ui.busy(false);
                        if !has_sessions { ui.status("Nenhuma sessão instalada. Instale Cinnamon, Xfce ou outra sessão do sistema.", true); }
                        else if ui.backend.is_demo() { ui.status("Prévia: senha monolito · código 000000", false); }
                        else { ui.status("Entre com sua conta para continuar.", false); }
                    }
                    Err(_) => { ui.status("LightDM indisponível. Inicie pelo gerenciador de login ou use --demo para visualizar.", true); ui.busy(false); }
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
                    self.status("Digite seu usuário local ou de domínio.", true);
                    self.username.grab_focus();
                    return;
                }
                let password = Zeroizing::new(self.password.text().to_string());
                self.password.set_text("");
                self.state.borrow_mut().begin(password.to_string());
                self.busy(true);
                self.status("Verificando suas credenciais…", false);
                self.cancel.set_label("Cancelar");
                if let Err(_) = self.backend.authenticate(user) {
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
            Phase::Prompt { .. } => {
                let response = Zeroizing::new(self.challenge.text().to_string());
                self.challenge.set_text("");
                self.challenge_box.set_visible(false);
                self.state.borrow_mut().phase = Phase::Authenticating;
                self.busy(true);
                self.status("Verificando resposta…", false);
                if self.backend.respond(&response).is_err() {
                    self.cancel_attempt("Falha ao enviar a resposta ao LightDM.");
                }
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
                    self.challenge_label.set_text(&text);
                    self.challenge.set_visibility(!secret);
                    self.challenge.set_text("");
                    self.challenge_box.set_visible(true);
                    self.spinner.set_spinning(false);
                    self.spinner.set_visible(false);
                    self.submit.set_sensitive(true);
                    self.challenge.grab_focus();
                    self.status("Conclua a verificação para continuar.", false);
                }
            }
            Event::Message(text, error) => {
                if matches!(
                    self.state.borrow().phase,
                    Phase::Authenticating | Phase::Prompt { .. }
                ) {
                    // Messages are never prompts. In particular they must not
                    // trigger respond("") and desynchronize a PAM conversation.
                    let warning = text.to_lowercase();
                    if error || !(warning.contains("arcfour") && warning.contains("deprecated")) {
                        self.status(&text, error);
                    }
                }
            }
            Event::Complete(authenticated) => {
                if self.state.borrow().phase == Phase::Cancelling {
                    self.failed("Autenticação cancelada. Você pode tentar novamente.");
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
                self.challenge_box.set_visible(false);
                if authenticated {
                    self.start_session();
                } else {
                    self.failed(
                        "Não foi possível entrar. Confira suas credenciais e tente novamente.",
                    );
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
            self.failed("Selecione uma sessão válida para continuar.");
            return;
        };
        self.state.borrow_mut().phase = Phase::Session;
        self.busy(true);
        self.cancel.set_sensitive(false);
        self.status("Preparando seu espaço…", false);
        let weak = Rc::downgrade(self);
        self.backend.start_session(&key, move |result| {
            if let Some(ui) = weak.upgrade() {
                match result {
                    Ok(()) => {
                        ui.state.borrow_mut().phase = Phase::Complete;
                        ui.spinner.set_spinning(false); ui.spinner.set_visible(false);
                        ui.status(if ui.backend.is_demo() { "Prévia concluída. Nenhuma sessão real foi aberta." } else { "Sessão iniciada." }, false);
                        if ui.backend.is_demo() { ui.cancel.set_sensitive(true); ui.cancel.set_label("Voltar à prévia"); }
                    }
                    Err(_) => ui.failed("A autenticação funcionou, mas a sessão não abriu. Confira os logs do LightDM e tente novamente."),
                }
            }
        });
    }
    fn failed(&self, message: &str) {
        self.clear_secrets();
        self.challenge_box.set_visible(false);
        self.busy(false);
        self.cancel.set_sensitive(true);
        self.cancel.set_label("Voltar");
        self.status(message, true);
        self.password.grab_focus();
    }
    fn cancel_attempt(&self, message: &str) {
        self.state.borrow_mut().cancel();
        self.password.set_text("");
        self.challenge.set_text("");
        self.challenge_box.set_visible(false);
        self.busy(true);
        self.status(message, true);
        if self.backend.cancel().is_err() {
            self.connected.set(false);
            self.busy(false);
            self.status("LightDM não confirmou o cancelamento. Reinicie o greeter antes de tentar novamente.", true);
        }
    }
    fn back(&self) {
        let phase = self.state.borrow().phase;
        match phase {
            Phase::Session | Phase::Cancelling => return,
            Phase::Authenticating | Phase::Prompt { .. } => { /* borrow ends below */ }
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
        popover.add_css_class("glass-popover");
        let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.append(&label("Acessibilidade", "popover-title"));
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
                        stack.set_transition_duration(if switch.is_active() { 0 } else { 280 });
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
        popover.add_css_class("glass-popover");
        let content = gtk::Box::new(gtk::Orientation::Vertical, 4);
        content.set_margin_top(8);
        content.set_margin_bottom(8);
        content.set_margin_start(8);
        content.set_margin_end(8);
        for power in [Power::Suspend, Power::Restart, Power::Shutdown] {
            let action = gtk::Button::with_label(power.label());
            action.add_css_class("power-action");
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
                            "Modo de prévia: nenhuma ação será executada."
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

/// Drives the actual GTK widgets and demo signal adapter, including a failed
/// password and a separate MFA challenge. It never connects to a real seat.
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
            2 if phase == (Phase::Prompt { secret: true }) => {
                if !ui.challenge_label.text().contains("000000") || !ui.password.text().is_empty() {
                    Some(false)
                } else {
                    ui.challenge.set_text("000000");
                    ui.submit.emit_clicked();
                    step.set(3);
                    None
                }
            }
            3 if phase == Phase::Complete => {
                if !ui.password.text().is_empty() || !ui.challenge.text().is_empty() {
                    Some(false)
                } else {
                    ui.back();
                    ui.reveal();
                    ui.username.set_text("teste@acme.example");
                    ui.password.set_text("monolito");
                    ui.submit.emit_clicked();
                    step.set(4);
                    None
                }
            }
            4 if phase == (Phase::Prompt { secret: true }) => {
                ui.challenge.set_text("partial-code");
                ui.back();
                step.set(5);
                None
            }
            5 if phase == Phase::Idle => {
                let cleared = ui.password.text().is_empty() && ui.challenge.text().is_empty();
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
            println!("GTK smoke: {} (senha inválida, nova tentativa AD, aviso ARCFOUR, senha, MFA, sessão simulada, cancelamento, limpeza de campos)", if passed { "PASS" } else { "FAIL" });
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
