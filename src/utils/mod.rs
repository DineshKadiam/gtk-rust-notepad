pub mod recolor;

pub(crate) mod scheme {
    use sourceview5::{StyleScheme, StyleSchemeManager};

    pub fn is_dark(scheme: &StyleScheme) -> Option<bool> {
        if let Some(variant) = scheme.metadata("variant") {
            Some(variant.contains("dark"))
        } else if scheme.name().contains("dark") {
            Some(true)
        } else {
            None
        }
    }

    /// Gets the dark variant of a StyleScheme or the scheme itself if it doesn't have a dark variant
    pub fn dark(scheme: &StyleScheme) -> StyleScheme {
        if let Some(dark) = is_dark(scheme)
            && !dark
        {
            // is a light variant
            StyleSchemeManager::default()
                .scheme(&scheme.metadata("dark-variant").unwrap())
                .unwrap()
        } else {
            scheme.clone()
        }
    }

    /// Gets the light variant of a StyleScheme or the scheme itself if it doesn't have a light variant
    pub fn light(scheme: &StyleScheme) -> StyleScheme {
        if let Some(dark) = is_dark(scheme)
            && dark
        {
            // is a light variant
            StyleSchemeManager::default()
                .scheme(&scheme.metadata("light-variant").unwrap())
                .unwrap()
        } else {
            scheme.clone()
        }
    }
}
