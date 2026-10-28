use std::collections::HashMap;

use futures::Stream;
use niri_ipc::{
    Action, Event, Output, Reply, Request, SizeChange, Workspace, WorkspaceReferenceArg,
    socket::Socket,
};
pub use state::{Snapshot, Window};
pub use window_stream::WindowStream;

use crate::error::Error;

mod reply;
mod state;
mod window_stream;

/// The top level client for Niri.
#[derive(Debug, Clone, Copy)]
pub struct Niri {}

impl Niri {
    pub fn new() -> Self {
        // Since niri_ipc is essentially stateless, we don't maintain anything much here.
        Self {}
    }

    /// Requests that the given window ID should be activated.
    #[tracing::instrument(level = "TRACE", err)]
    pub fn activate_window(&self, id: u64) -> Result<(), Error> {
        let reply = request(Request::Action(Action::FocusWindow { id }))?;
        reply::typed!(Handled, reply)
    }

    /// Requests that the given window ID should be closed.
    #[tracing::instrument(level = "TRACE", err)]
    pub fn close_window(&self, id: u64) -> Result<(), Error> {
        let reply = request(Request::Action(Action::CloseWindow { id: Some(id) }))?;
        reply::typed!(Handled, reply)
    }

    /// Moves the column containing the given window to the given 1-based column index, optionally
    /// moving it to another workspace first.
    ///
    /// Niri only supports moving the focused column, so this focuses the window.
    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_column(
        &self,
        window_id: u64,
        workspace_id: Option<u64>,
        index: usize,
    ) -> Result<(), Error> {
        self.activate_window(window_id)?;

        if let Some(workspace_id) = workspace_id {
            let reply = request(Request::Action(Action::MoveColumnToWorkspace {
                reference: WorkspaceReferenceArg::Id(workspace_id),
                focus: true,
            }))?;
            reply::typed!(Handled, reply)?;
        }

        let reply = request(Request::Action(Action::MoveColumnToIndex { index }))?;
        reply::typed!(Handled, reply)
    }

    /// Moves the given window to another workspace without changing focus.
    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_window_to_workspace(&self, window_id: u64, workspace_id: u64) -> Result<(), Error> {
        let reply = request(Request::Action(Action::MoveWindowToWorkspace {
            window_id: Some(window_id),
            reference: WorkspaceReferenceArg::Id(workspace_id),
            focus: false,
        }))?;
        reply::typed!(Handled, reply)
    }

    /// Sets the width of the given window as a proportion (0, 1] of the working area.
    #[tracing::instrument(level = "TRACE", err)]
    pub fn set_window_width(&self, id: u64, proportion: f64) -> Result<(), Error> {
        // Niri expects proportions as percentages over IPC.
        let reply = request(Request::Action(Action::SetWindowWidth {
            id: Some(id),
            change: SizeChange::SetProportion(proportion * 100.0),
        }))?;
        reply::typed!(Handled, reply)
    }

    /// Returns the current outputs.
    pub fn outputs(&self) -> Result<HashMap<String, Output>, Error> {
        let reply = request(Request::Outputs)?;
        reply::typed!(Outputs, reply)
    }

    /// Returns a stream of window snapshots.
    pub fn window_stream(&self) -> WindowStream {
        WindowStream::new()
    }

    /// Returns a stream of workspace changes.
    pub fn workspace_stream(&self) -> Result<impl Stream<Item = Vec<Workspace>> + use<>, Error> {
        let mut socket = socket()?;
        let reply = socket.send(Request::EventStream).map_err(Error::NiriIpc)?;
        reply::typed!(Handled, reply)?;

        let mut next = socket.read_events();
        Ok(async_stream::stream! {
            loop {
                match next() {
                    Ok(Event::WorkspacesChanged { workspaces }) => {
                        yield workspaces;
                    }
                    Ok(_) => (),
                    Err(e) => {
                        tracing::error!(%e, "Niri IPC error reading from event stream");
                    }
                }
            }
        })
    }
}

// Helper to marshal request errors into our own type system.
//
// This can't be used for event streams, since the stream callback is thrown away in this function.
#[tracing::instrument(level = "TRACE", err)]
fn request(request: Request) -> Result<Reply, Error> {
    socket()?.send(request).map_err(Error::NiriIpc)
}

// Helper to connect to the Niri socket.
#[tracing::instrument(level = "TRACE", err)]
fn socket() -> Result<Socket, Error> {
    Socket::connect().map_err(Error::NiriIpc)
}
