use std::path::PathBuf;

pub struct Config {
    pub title: String,
    pub domain_hint: String,
    pub wallpaper: Option<PathBuf>,
    pub auto_ack: bool,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let path = std::env::var_os("MONOLITO_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|| "/etc/monolito-greeter/config.ini".into());
        let file = glib::KeyFile::new();
        if path.exists() {
            file.load_from_file(&path, glib::KeyFileFlags::NONE)
                .map_err(|e| format!("Configuração inválida: {e}"))?;
        } else if std::env::var_os("MONOLITO_CONFIG").is_some() {
            return Err("O arquivo MONOLITO_CONFIG não existe.".into());
        }
        let wallpaper = file
            .string("Greeter", "wallpaper")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);
        if let Some(path) = &wallpaper {
            if !path.is_absolute() || !path.is_file() {
                return Err("O wallpaper deve ser um arquivo absoluto legível.".into());
            }
        }
        Ok(Self {
            title: file
                .string("Greeter", "title")
                .map(|s| s.to_string())
                .unwrap_or_else(|_| "Outro usuário".into()),
            domain_hint: file
                .string("Greeter", "domain-hint")
                .map(|s| s.to_string())
                .unwrap_or_else(|_| "Conecte-se com sua conta corporativa ou local".into()),
            wallpaper,
            auto_ack: file
                .boolean("Greeter", "acknowledge-arcfour")
                .unwrap_or(true),
        })
    }
}
