use crate::as_dev_err;
use axdriver_base::{BaseDriverOps, DevResult, DeviceType};
use axdriver_vsock::{VsockDriverEvent, VsockDriverOps};
use virtio_drivers::device::socket::{
    SocketError, VirtIOSocket, VsockAddr, VsockConnectionManager as InnerDev, VsockEvent,
    VsockEventType,
};
use virtio_drivers::{Error as VirtIoError, Hal, transport::Transport};
extern crate alloc;

/// The VirtIO socket device driver.
pub struct VirtIoSocketDev<H: Hal, T: Transport> {
    inner: InnerDev<H, T>,
}

unsafe impl<H: Hal, T: Transport> Send for VirtIoSocketDev<H, T> {}
unsafe impl<H: Hal, T: Transport> Sync for VirtIoSocketDev<H, T> {}

impl<H: Hal, T: Transport> VirtIoSocketDev<H, T> {
    /// Creates a new driver instance and initializes the device, or returns
    /// an error if any step fails.
    pub fn try_new(transport: T) -> DevResult<Self> {
        let viotio_socket = VirtIOSocket::<H, _>::new(transport).map_err(as_dev_err)?;
        Ok(Self {
            inner: InnerDev::new(viotio_socket),
        })
    }
}

impl<H: Hal, T: Transport> BaseDriverOps for VirtIoSocketDev<H, T> {
    fn device_name(&self) -> &str {
        "virtio-vsocket"
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Vsock
    }
}

impl<H: Hal, T: Transport> VsockDriverOps for VirtIoSocketDev<H, T> {
    fn guest_cid(&self) -> u32 {
        self.inner.guest_cid() as u32
    }

    fn listen(&mut self, src_port: u32) {
        self.inner.listen(src_port)
    }

    fn connect(&mut self, peer_cid: u32, peer_port: u32, src_port: u32) -> DevResult<()> {
        self.inner
            .connect(
                VsockAddr {
                    cid: peer_cid as u64,
                    port: peer_port,
                },
                src_port,
            )
            .map_err(as_dev_err)
    }

    fn send(
        &mut self,
        peer_cid: u32,
        peer_port: u32,
        src_port: u32,
        buf: &[u8],
    ) -> DevResult<usize> {
        match self.inner.send(
            VsockAddr {
                cid: peer_cid as u64,
                port: peer_port,
            },
            src_port,
            buf,
        ) {
            Ok(()) => Ok(buf.len()),
            Err(e) => Err(as_dev_err(e)),
        }
    }

    fn recv(
        &mut self,
        peer_cid: u32,
        peer_port: u32,
        src_port: u32,
        buf: &mut [u8],
    ) -> DevResult<usize> {
        self.inner
            .recv(
                VsockAddr {
                    cid: peer_cid as u64,
                    port: peer_port,
                },
                src_port,
                buf,
            )
            .map_err(as_dev_err)
    }

    fn recv_avail(&mut self, peer_cid: u32, peer_port: u32, src_port: u32) -> DevResult<usize> {
        self.inner
            .recv_buffer_available_bytes(
                VsockAddr {
                    cid: peer_cid as u64,
                    port: peer_port,
                },
                src_port,
            )
            .map_err(as_dev_err)
    }

    fn disconnect(&mut self, peer_cid: u32, peer_port: u32, src_port: u32) -> DevResult<()> {
        self.inner
            .shutdown(
                VsockAddr {
                    cid: peer_cid as u64,
                    port: peer_port,
                },
                src_port,
            )
            .map_err(as_dev_err)
    }

    fn abort(&mut self, peer_cid: u32, peer_port: u32, src_port: u32) -> DevResult<()> {
        self.inner
            .force_close(
                VsockAddr {
                    cid: peer_cid as u64,
                    port: peer_port,
                },
                src_port,
            )
            .map_err(as_dev_err)
    }

    fn poll_event(&mut self) -> DevResult<Option<VsockDriverEvent>> {
        match self.inner.poll() {
            Ok(None) => {
                // no event
                Ok(None)
            }
            Ok(Some(event)) => {
                // translate event
                let result = convert_vsock_event(event, &mut self.inner)?;
                Ok(Some(result))
            }
            Err(e) => {
                // error
                Err(as_dev_err(e))
            }
        }
    }
}

fn convert_vsock_event<H: Hal, T: Transport>(
    event: VsockEvent,
    inner: &mut InnerDev<H, T>,
) -> DevResult<VsockDriverEvent> {
    let local_port = event.destination.port;
    let peer_cid = event.source.cid as u32;
    let peer_port = event.source.port;

    match event.event_type {
        VsockEventType::ConnectionRequest => Ok(VsockDriverEvent::ConnectionRequest(
            local_port, peer_cid, peer_port,
        )),
        VsockEventType::Connected => {
            Ok(VsockDriverEvent::Connected(local_port, peer_cid, peer_port))
        }
        VsockEventType::Received { length } => {
            let mut data = alloc::vec![0u8; length];
            let read = inner
                .recv(
                    VsockAddr {
                        cid: peer_cid as u64,
                        port: peer_port,
                    },
                    local_port,
                    &mut data,
                )
                .map_err(as_dev_err)?;
            data.truncate(read);
            Ok(VsockDriverEvent::DataReceived(
                local_port, peer_cid, peer_port, data,
            ))
        }
        VsockEventType::Disconnected { reason: _ } => Ok(VsockDriverEvent::Disconnect(
            local_port, peer_cid, peer_port,
        )),
        _ => Ok(VsockDriverEvent::Unknown),
    }
}
