#[derive(Default)]
pub struct ClipboardService(Option<arboard::Clipboard>);

#[derive(Debug)]
pub struct ClipboardUnavailable;

impl ClipboardService {
    /// This is called only in response to an explicit Copy action.
    pub fn copy(&mut self, text: &str) -> Result<(), ClipboardUnavailable> {
        if self.0.is_none() {
            self.0 = Some(arboard::Clipboard::new().map_err(|_| ClipboardUnavailable)?);
        }
        self.0
            .as_mut()
            .ok_or(ClipboardUnavailable)?
            .set_text(text)
            .map_err(|_| ClipboardUnavailable)
    }
}
