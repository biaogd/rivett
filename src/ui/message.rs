use crate::terminal::TerminalDamage;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveView {
    Terminal,
    SessionManager,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    // CreateSession, // Removed unused
    CreateLocalTab,
    SelectTab(usize),
    CloseTab(usize),
    ToggleMenu,
    // Menu actions
    ShowSessionManager,
    ToggleSftpPanel,
    SftpDragStart,
    SftpDragEnd,
    SftpDragMove(iced::Point),
    SftpLocalPathChanged(String),
    SftpRemotePathChanged(String),
    ShowPortForwarding,
    ShowSettings,
    // Quick Connect
    ToggleQuickConnect,
    QuickConnectQueryChanged(String),
    SelectQuickConnectSession(String), // Session Name
    ToggleSessionMenu(String),
    CloseSessionMenu,
    // Session management
    CreateNewSession,
    EditSession(String),
    DeleteSession(String),
    ConnectToSession(String),
    SaveSession,
    CancelSessionEdit,
    CloseSessionManager,
    ToggleAuthMethod,
    #[allow(dead_code)]
    ClearValidationError,
    // Session form fields
    SessionNameChanged(String),
    SessionHostChanged(String),
    SessionPortChanged(String),
    SessionUsernameChanged(String),
    SessionPasswordChanged(String),
    TogglePasswordVisibility,
    SessionKeyPathChanged(String),
    SessionKeyPassphraseChanged(String),
    SessionSearchChanged(String),
    TestConnection,
    TestConnectionResult(Result<(), String>),
    // SSH Connection
    SessionConnected(
        Result<
            (
                Arc<Mutex<crate::ssh::SshSession>>,
                Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>>,
            ),
            String,
        >,
        usize,
    ),
    ShellOpened(Result<russh::ChannelId, String>, usize),
    TerminalDataReceived(usize, Vec<u8>),
    TerminalDamaged(usize, TerminalDamage),
    TerminalInput(Vec<u8>),
    // Terminal Mouse Events
    TerminalMousePress(usize, usize),
    TerminalMouseDrag(usize, usize),
    TerminalMouseRelease,
    TerminalMouseDoubleClick(usize, usize),
    TerminalResize(usize, usize),
    WindowResized(u32, u32),
    WindowOpened(iced::window::Id),
    WindowClosed(iced::window::Id),
    ScrollWheel(f32),         // delta in lines
    RetryConnection(usize),   // tab index to retry
    EditSessionConfig(usize), // tab index to edit
    Copy,
    Paste,
    ClipboardReceived(Option<String>),
    ImeBufferChanged(String),
    ImeFocusChanged(bool),
    ImePaste,
    RuntimeEvent(iced::event::Event, iced::window::Id),
    Ignore,
    Tick(std::time::Instant),
}
