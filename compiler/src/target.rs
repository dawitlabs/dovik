#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Target {
    #[default]
    Native,
    Wasi,
}

impl Target {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "native" | "x86_64-linux" => Some(Self::Native),
            "wasm" | "wasm32-wasip1" => Some(Self::Wasi),
            _ => None,
        }
    }

    pub fn triple(self) -> &'static str {
        match self {
            Self::Native => "x86_64-pc-linux-gnu",
            Self::Wasi => "wasm32-unknown-wasip1",
        }
    }

    pub fn entry_symbol(self) -> &'static str {
        match self {
            Self::Native => "main",
            Self::Wasi => "__main_argc_argv",
        }
    }

    pub fn needs_depth_guard(self) -> bool {
        self == Self::Wasi
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Native => "",
            Self::Wasi => ".wasm",
        }
    }
}
