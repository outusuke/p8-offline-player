//! Sanity checks for the runtime file the user imports.

const MIN_LEN: usize = 200_000;
const MAX_LEN: usize = 30_000_000;

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

// Catch the usual wrong picks (a cart PNG, a saved HTML page) before they replace a working runtime.
pub fn validate(data: &[u8]) -> Result<(), String> {
    if data.starts_with(b"\x89PNG") {
        return Err("That is a PNG (a cart). Select the runtime .js file instead.".into());
    }
    let head = &data[..data.len().min(512)];
    let head_trim: Vec<u8> = head.iter().copied().skip_while(|b| b.is_ascii_whitespace()).collect();
    if head_trim.starts_with(b"<") {
        return Err("That looks like an HTML page. Save the runtime .js file itself (e.g. pico8_0207.js), not the page.".into());
    }
    if data.len() < MIN_LEN || data.len() > MAX_LEN {
        return Err(format!(
            "That file is {} bytes; the PICO-8 web runtime is a ~1.5 MB .js file.",
            data.len()
        ));
    }
    if data.starts_with(b"PK") || data[..MIN_LEN].contains(&0) {
        return Err("That is a binary file (an app or archive), not the runtime .js file.".into());
    }
    if !contains(&data[..200], b"Module") || !contains(data, b"pico8_state") {
        return Err("That .js file does not look like the PICO-8 web runtime.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake(len: usize) -> Vec<u8> {
        let mut v = b"var Module=typeof Module!='undefined'?Module:{};/*pico8_state*/".to_vec();
        v.resize(len, b' ');
        v
    }

    #[test]
    fn accepts_plausible_runtime() {
        assert!(validate(&fake(1_600_000)).is_ok());
    }

    #[test]
    fn rejects_wrong_files() {
        assert!(validate(b"\x89PNG\r\n\x1a\n....").unwrap_err().contains("PNG"));
        let mut html = b"  <!DOCTYPE html>".to_vec();
        html.resize(1_600_000, b' ');
        assert!(validate(&html).unwrap_err().contains("HTML"));
        assert!(validate(&fake(1000)).is_err());
        let mut apk = b"PK\x03\x04".to_vec();
        apk.resize(1_600_000, 0);
        assert!(validate(&apk).unwrap_err().contains("binary"));
        let mut late = b"console.log(1)".to_vec();
        late.resize(1_600_000, b' ');
        late.extend_from_slice(b"Module pico8_state");
        assert!(validate(&late).is_err());
        let mut other = b"console.log(1)".to_vec();
        other.resize(1_600_000, b' ');
        assert!(validate(&other).unwrap_err().contains("does not look"));
    }
}
