//! Common traits and types for socket communite device drivers (i.e. disk).

#![no_std]
#![cfg_attr(doc, feature(doc_auto_cfg))]

#[doc(no_inline)]
pub use axdriver_base::{BaseDriverOps, DevError, DevResult, DeviceType};

extern crate alloc;

/// VsockDriverEvent
#[derive(Debug)]
pub enum VsockDriverEvent {
    /// ConnectionRequest with local_port, peer_cid, peer_port
    ConnectionRequest(u32, u32, u32),
    /// Connected with local_port, peer_cid, peer_port
    Connected(u32, u32, u32),
    /// Receive with local_port, peer_cid, peer_por, data length
    DataReceived(u32, u32, u32, alloc::vec::Vec<u8>),
    /// Disconnect with local_port, peer_cid, peer_port
    Disconnect(u32, u32, u32),
    /// unknown event
    Unknown,
}

/// Operations that require a block storage device driver to implement.
pub trait VsockDriverOps: BaseDriverOps {
    /// guest cid
    fn guest_cid(&self) -> u32;

    /// Listen on a specific port.
    fn listen(&mut self, src_port: u32);

    /// Connect to a peer socket.
    fn connect(&mut self, peer_cid: u32, peer_port: u32, src_port: u32) -> DevResult<()>;

    /// Send data to the connected peer socket. need addr for DGRAM mode
    fn send(
        &mut self,
        peer_cid: u32,
        peer_port: u32,
        src_port: u32,
        buf: &[u8],
    ) -> DevResult<usize>;

    /// Receive data from the connected peer socket.
    fn recv(
        &mut self,
        peer_cid: u32,
        peer_port: u32,
        src_port: u32,
        buf: &mut [u8],
    ) -> DevResult<usize>;

    /// Returns the number of bytes in the receive buffer available to be read by recv.
    fn recv_avail(&mut self, peer_cid: u32, peer_port: u32, src_port: u32) -> DevResult<usize>;

    /// Disconnect from the connected peer socket.
    ///
    /// Requests to shut down the connection cleanly, telling the peer that we won't send or receive
    /// any more data.
    fn disconnect(&mut self, peer_cid: u32, peer_port: u32, src_port: u32) -> DevResult<()>;

    /// Forcibly closes the connection without waiting for the peer.
    fn abort(&mut self, peer_cid: u32, peer_port: u32, src_port: u32) -> DevResult<()>;

    /// poll event from driver
    fn poll_event(&mut self) -> DevResult<Option<VsockDriverEvent>>;
}
