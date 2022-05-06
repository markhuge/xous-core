pub(crate) const SERVER_NAME_USBTEST: &'static str = "_USB test and development server_";

#[derive(num_derive::FromPrimitive, num_derive::ToPrimitive, Debug)]
pub(crate) enum Opcode {
    /// Handle the USB interrupt
    UsbIrqHandler,
    /// Handle a USB reset - needed because the API doesn't allow mutability in the trait :-/
    UsbReset,
    /// Parse a commandlne
    DoCmd,
    /// Keyboard input
    KeyboardChar,
    /// Keyboard handler input
    HandlerTrigger,
    /// Suspend/resume callback
    SuspendResume,
    /// Exits the server
    Quit,
}
