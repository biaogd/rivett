use crate::terminal::TerminalDamage;
use crate::ui::state::{PortForwardStatus, SftpContextAction, SftpPane, SftpTransferUpdate};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveView {
    Terminal,
    SessionManager,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionDialogTab {
    General,
    PortForwarding,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    // CreateSession, // Removed unused
    CreateLocalTab,
    SelectTab(usize),
    CloseTab(usize),
    // Menu actions
    ShowSessionManager,
    ToggleSftpPanel,
    TogglePortForwardPanel,
    ApplyPortForwards,
    PortForwardStatusUpdated(String, Vec<(String, PortForwardStatus)>),
    PortForwardDragStart,
    PortForwardDragEnd,
    PortForwardDragMove(iced::Point),
    SftpDragStart,                             // Window resize drag
    SftpDragEnd,                               // Window resize drag end
    SftpDragMove(iced::Point),                 // Window resize drag move
    SftpFileDragStart(SftpPane, String),       // Internal file drag start
    SftpFileDragUpdate(iced::Point),           // Internal file drag update
    SftpFileDragEnd,                           // Internal file drag end
    SftpFileHover(Option<(SftpPane, String)>), // Hover state
    SftpLocalPathChanged(String),
    SftpRemotePathChanged(String),
    SftpRemoteLoaded(
        usize,
        Result<(Vec<crate::ui::state::SftpEntry>, Option<String>), String>,
    ),
    SftpPanelCursorMoved(iced::Point),
    SftpOpenContextMenu(SftpPane, String),
    SftpCloseContextMenu,
    SftpContextAction(SftpPane, String, SftpContextAction),
    SftpTransferUpdate(SftpTransferUpdate),
    SftpTransferCancel(Uuid),
    SftpTransferRetry(Uuid),
    SftpTransferClearDone,
    SftpTransferPause(Uuid),
    SftpTransferResume(Uuid),
    SftpRenameStart(SftpPane, String, bool),
    SftpRenameInput(String),
    SftpRenameCancel,
    SftpRenameConfirm,
    SftpRenameFinished(usize, Result<(), String>),
    SftpDeleteStart(SftpPane, String, bool),
    SftpDeleteCancel,
    SftpDeleteConfirm,
    SftpDeleteFinished(usize, Result<(), String>),
    SftpLocalEntryPressed(String, bool),
    SftpRemoteEntryPressed(String, bool),
    OpenPortForwarding(String),
    ClosePortForwarding,
    PortForwardLocalPortChanged(String),
    PortForwardLocalHostChanged(String),
    PortForwardRemoteHostChanged(String),
    PortForwardRemotePortChanged(String),
    AddPortForward,
    TogglePortForward(String),
    DeletePortForward(String),
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
    SessionKeyIdChanged(String),
    SessionKeyPassphraseChanged(String),
    SessionSearchChanged(String),
    ToggleSavedKeyMenu,
    CloseSavedKeyMenu,
    SessionDialogTabSelected(SessionDialogTab),
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
    OpenUrl(String),
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
