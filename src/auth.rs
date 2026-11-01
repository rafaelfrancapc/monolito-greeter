//! Small, owned binding to the system liblightdm-gobject ABI. All GTK and
//! LightDM callbacks run on the GLib main context. No PAM work runs in the UI.
use glib::{prelude::*, translate::*};
use std::{
    cell::{Cell, RefCell},
    ffi::{CStr, CString},
    os::raw::c_char,
    ptr,
    rc::Rc,
};
use zeroize::{Zeroize, Zeroizing};

type Object = glib::gobject_ffi::GObject;
type Error = glib::ffi::GError;

#[link(name = "lightdm-gobject-1")]
extern "C" {
    fn lightdm_greeter_new() -> *mut Object;
    fn lightdm_greeter_connect_to_daemon(
        g: *mut Object,
        c: *mut gio::ffi::GCancellable,
        cb: gio::ffi::GAsyncReadyCallback,
        data: glib::ffi::gpointer,
    );
    fn lightdm_greeter_connect_to_daemon_finish(
        g: *mut Object,
        r: *mut gio::ffi::GAsyncResult,
        e: *mut *mut Error,
    ) -> i32;
    fn lightdm_greeter_authenticate(g: *mut Object, user: *const c_char, e: *mut *mut Error)
        -> i32;
    fn lightdm_greeter_respond(g: *mut Object, response: *const c_char, e: *mut *mut Error) -> i32;
    fn lightdm_greeter_cancel_authentication(g: *mut Object, e: *mut *mut Error) -> i32;
    fn lightdm_greeter_get_is_authenticated(g: *mut Object) -> i32;
    fn lightdm_greeter_get_default_session_hint(g: *mut Object) -> *const c_char;
    fn lightdm_greeter_get_select_user_hint(g: *mut Object) -> *const c_char;
    fn lightdm_greeter_start_session(
        g: *mut Object,
        session: *const c_char,
        c: *mut gio::ffi::GCancellable,
        cb: gio::ffi::GAsyncReadyCallback,
        data: glib::ffi::gpointer,
    );
    fn lightdm_greeter_start_session_finish(
        g: *mut Object,
        r: *mut gio::ffi::GAsyncResult,
        e: *mut *mut Error,
    ) -> i32;
    fn lightdm_get_sessions() -> *mut glib::ffi::GList;
    fn lightdm_session_get_key(session: *mut Object) -> *const c_char;
    fn lightdm_session_get_name(session: *mut Object) -> *const c_char;
    fn lightdm_get_can_shutdown() -> i32;
    fn lightdm_get_can_restart() -> i32;
    fn lightdm_get_can_suspend() -> i32;
    fn lightdm_shutdown(e: *mut *mut Error) -> i32;
    fn lightdm_restart(e: *mut *mut Error) -> i32;
    fn lightdm_suspend(e: *mut *mut Error) -> i32;
}

#[derive(Clone, Debug)]
pub enum Event {
    Prompt(String, bool),
    Message(String, bool),
    Complete(bool),
    Reset,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub key: String,
    pub name: String,
}

#[derive(Clone, Copy)]
pub enum Power {
    Suspend,
    Restart,
    Shutdown,
}

impl Power {
    pub fn label(self) -> &'static str {
        match self {
            Self::Suspend => "Suspender",
            Self::Restart => "Reiniciar",
            Self::Shutdown => "Desligar",
        }
    }
}

fn checked(ok: i32, error: *mut Error) -> Result<(), String> {
    if !error.is_null() {
        // The API transfers ownership of GError to its caller.
        let error: glib::Error = unsafe { from_glib_full(error) };
        Err(error.to_string())
    } else if ok != 0 {
        Ok(())
    } else {
        Err("O LightDM não concluiu a operação.".into())
    }
}

unsafe fn text(pointer: *const c_char) -> String {
    if pointer.is_null() {
        String::new()
    } else {
        CStr::from_ptr(pointer).to_string_lossy().into_owned()
    }
}

#[derive(Clone)]
pub struct Native {
    object: glib::Object,
}

enum Operation {
    Connect,
    StartSession,
}
struct AsyncCall {
    native: Native,
    operation: Operation,
    done: Box<dyn FnOnce(Result<(), String>)>,
}

unsafe extern "C" fn finished(
    _: *mut Object,
    result: *mut gio::ffi::GAsyncResult,
    data: glib::ffi::gpointer,
) {
    let call = Box::from_raw(data as *mut AsyncCall);
    let mut error = ptr::null_mut();
    let ok = match call.operation {
        Operation::Connect => {
            lightdm_greeter_connect_to_daemon_finish(call.native.ptr(), result, &mut error)
        }
        Operation::StartSession => {
            lightdm_greeter_start_session_finish(call.native.ptr(), result, &mut error)
        }
    };
    (call.done)(checked(ok, error));
}

impl Native {
    pub fn new() -> Result<Self, String> {
        let pointer = unsafe { lightdm_greeter_new() };
        if pointer.is_null() {
            return Err("Não foi possível criar o cliente LightDM.".into());
        }
        Ok(Self {
            object: unsafe { from_glib_full(pointer) },
        })
    }
    fn ptr(&self) -> *mut Object {
        self.object.as_ptr()
    }

    fn connect(&self, done: impl FnOnce(Result<(), String>) + 'static) {
        let call = Box::new(AsyncCall {
            native: self.clone(),
            operation: Operation::Connect,
            done: Box::new(done),
        });
        unsafe {
            lightdm_greeter_connect_to_daemon(
                self.ptr(),
                ptr::null_mut(),
                Some(finished),
                Box::into_raw(call).cast(),
            );
        }
    }

    fn subscribe(&self, emit: Rc<dyn Fn(Event)>) {
        let handler = emit.clone();
        self.object
            .connect_local("show-prompt", false, move |values| {
                if let Ok(message) = values[1].get::<String>() {
                    let kind =
                        unsafe { glib::gobject_ffi::g_value_get_enum(values[2].to_glib_none().0) };
                    handler(Event::Prompt(message, kind == 1));
                }
                None
            });
        let handler = emit.clone();
        self.object
            .connect_local("show-message", false, move |values| {
                if let Ok(message) = values[1].get::<String>() {
                    let kind =
                        unsafe { glib::gobject_ffi::g_value_get_enum(values[2].to_glib_none().0) };
                    handler(Event::Message(message, kind == 1));
                }
                None
            });
        let object = self.object.downgrade();
        let handler = emit.clone();
        self.object
            .connect_local("authentication-complete", false, move |_| {
                if let Some(object) = object.upgrade() {
                    handler(Event::Complete(unsafe {
                        lightdm_greeter_get_is_authenticated(object.as_ptr()) != 0
                    }));
                }
                None
            });
        self.object.connect_local("reset", false, move |_| {
            emit(Event::Reset);
            None
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_gobject_signals_preserve_prompt_and_message_types() {
        let native = Native::new().expect("system liblightdm object");
        let events = Rc::new(RefCell::new(Vec::new()));
        let received = events.clone();
        native.subscribe(Rc::new(move |event| received.borrow_mut().push(event)));
        // Emit the real C GObject signals, not the demo adapter. This verifies
        // the FFI enum ABI, ownership and Rust signal conversion without a seat.
        unsafe {
            for (signal, message, kind) in [
                ("show-prompt", "Password:", 1i32),
                ("show-prompt", "Press Enter", 0i32),
                ("show-message", "Notice", 0i32),
                ("show-message", "Failure", 1i32),
            ] {
                let signal = CString::new(signal).expect("literal");
                let message = CString::new(message).expect("literal");
                glib::gobject_ffi::g_signal_emit_by_name(
                    native.ptr(),
                    signal.as_ptr(),
                    message.as_ptr(),
                    kind,
                );
            }
        }
        let events = events.borrow();
        assert!(matches!(&events[0], Event::Prompt(text, true) if text == "Password:"));
        assert!(matches!(&events[1], Event::Prompt(text, false) if text == "Press Enter"));
        assert!(matches!(&events[2], Event::Message(text, false) if text == "Notice"));
        assert!(matches!(&events[3], Event::Message(text, true) if text == "Failure"));
    }
}

type Handler = Rc<RefCell<Option<Rc<dyn Fn(Event)>>>>;

pub enum Backend {
    Native(Native),
    Demo { stage: Cell<u8>, handler: Handler },
}

impl Backend {
    pub fn new(demo: bool) -> Result<Self, String> {
        if demo {
            Ok(Self::Demo {
                stage: Cell::new(0),
                handler: Rc::new(RefCell::new(None)),
            })
        } else {
            Native::new().map(Self::Native)
        }
    }
    pub fn is_demo(&self) -> bool {
        matches!(self, Self::Demo { .. })
    }
    pub fn subscribe(&self, handler: Rc<dyn Fn(Event)>) {
        match self {
            Self::Native(native) => native.subscribe(handler),
            Self::Demo {
                handler: target, ..
            } => *target.borrow_mut() = Some(handler),
        }
    }
    fn emit(&self, event: Event) {
        if let Self::Demo { handler, .. } = self {
            let handler = handler.clone();
            glib::idle_add_local_once(move || {
                if let Some(handler) = handler.borrow().as_ref() {
                    handler(event);
                }
            });
        }
    }
    pub fn connect(&self, done: impl FnOnce(Result<(), String>) + 'static) {
        match self {
            Self::Native(native) => native.connect(done),
            Self::Demo { .. } => done(Ok(())),
        }
    }
    pub fn authenticate(&self, user: &str) -> Result<(), String> {
        match self {
            Self::Native(native) => {
                let user = CString::new(user).map_err(|_| "Usuário inválido.")?;
                let mut error = ptr::null_mut();
                checked(
                    unsafe {
                        lightdm_greeter_authenticate(native.ptr(), user.as_ptr(), &mut error)
                    },
                    error,
                )
            }
            Self::Demo { stage, .. } => {
                stage.set(1);
                self.emit(Event::Message(
                    "ARCFOUR encryption is deprecated".into(),
                    false,
                ));
                self.emit(Event::Prompt(
                    "ARCFOUR is deprecated. Press Enter to continue.".into(),
                    false,
                ));
                Ok(())
            }
        }
    }
    pub fn respond(&self, response: &str) -> Result<(), String> {
        match self {
            Self::Native(native) => {
                let mut bytes = Zeroizing::new(response.as_bytes().to_vec());
                if bytes.contains(&0) {
                    return Err("A resposta contém um caractere inválido.".into());
                }
                bytes.push(0);
                let mut error = ptr::null_mut();
                let ok = unsafe {
                    lightdm_greeter_respond(native.ptr(), bytes.as_ptr().cast(), &mut error)
                };
                bytes.zeroize();
                checked(ok, error)
            }
            Self::Demo { stage, .. } => {
                match stage.get() {
                    1 => {
                        stage.set(2);
                        self.emit(Event::Prompt("Password:".into(), true));
                    }
                    2 if response == "monolito" => {
                        stage.set(3);
                        self.emit(Event::Complete(true));
                    }
                    _ => {
                        stage.set(0);
                        self.emit(Event::Complete(false));
                    }
                }
                Ok(())
            }
        }
    }
    pub fn cancel(&self) -> Result<(), String> {
        match self {
            Self::Native(native) => {
                let mut error = ptr::null_mut();
                checked(
                    unsafe { lightdm_greeter_cancel_authentication(native.ptr(), &mut error) },
                    error,
                )
            }
            Self::Demo { stage, .. } => {
                stage.set(0);
                self.emit(Event::Complete(false));
                Ok(())
            }
        }
    }
    pub fn sessions(&self) -> Vec<Session> {
        if self.is_demo() {
            return vec![
                Session {
                    key: "cinnamon".into(),
                    name: "Cinnamon".into(),
                },
                Session {
                    key: "xfce".into(),
                    name: "Xfce".into(),
                },
            ];
        }
        let mut sessions = Vec::new();
        // transfer-none: the session list and its objects belong to LightDM.
        unsafe {
            let mut item = lightdm_get_sessions();
            while !item.is_null() {
                let session = (*item).data.cast();
                let key = text(lightdm_session_get_key(session));
                if !key.is_empty() {
                    sessions.push(Session {
                        key,
                        name: text(lightdm_session_get_name(session)),
                    });
                }
                item = (*item).next;
            }
        }
        sessions
    }
    pub fn default_session(&self) -> String {
        match self {
            Self::Native(native) => unsafe {
                text(lightdm_greeter_get_default_session_hint(native.ptr()))
            },
            Self::Demo { .. } => "xfce".into(),
        }
    }
    pub fn select_user_hint(&self) -> Option<String> {
        match self {
            Self::Native(native) => unsafe {
                let name = text(lightdm_greeter_get_select_user_hint(native.ptr()));
                if name.is_empty() {
                    None
                } else {
                    Some(name)
                }
            },
            Self::Demo { .. } => Some("rafael".into()),
        }
    }
    pub fn start_session(&self, session: &str, done: impl FnOnce(Result<(), String>) + 'static) {
        match self {
            Self::Native(native) => {
                let session = match CString::new(session) {
                    Ok(s) => s,
                    Err(_) => {
                        done(Err("Sessão inválida.".into()));
                        return;
                    }
                };
                let call = Box::new(AsyncCall {
                    native: native.clone(),
                    operation: Operation::StartSession,
                    done: Box::new(done),
                });
                unsafe {
                    lightdm_greeter_start_session(
                        native.ptr(),
                        session.as_ptr(),
                        ptr::null_mut(),
                        Some(finished),
                        Box::into_raw(call).cast(),
                    );
                }
            }
            Self::Demo { .. } => {
                glib::idle_add_local_once(move || done(Ok(())));
            }
        }
    }
    pub fn can_power(&self, power: Power) -> bool {
        if self.is_demo() {
            return true;
        }
        unsafe {
            (match power {
                Power::Suspend => lightdm_get_can_suspend(),
                Power::Restart => lightdm_get_can_restart(),
                Power::Shutdown => lightdm_get_can_shutdown(),
            }) != 0
        }
    }
    pub fn power(&self, power: Power) -> Result<(), String> {
        if self.is_demo() {
            return Ok(());
        }
        let mut error = ptr::null_mut();
        let ok = unsafe {
            match power {
                Power::Suspend => lightdm_suspend(&mut error),
                Power::Restart => lightdm_restart(&mut error),
                Power::Shutdown => lightdm_shutdown(&mut error),
            }
        };
        checked(ok, error)
    }
}
