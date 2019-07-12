use core::pin::Pin;
use core::task::{Context, Poll};

use futures::io::{AsyncRead, AsyncWrite, Error, Initializer};

use zephyr_core::context::Any as C;
use zephyr_core::poll::Signal;
use zephyr_futures::current_reactor_register;

use super::{UartBufferedRx, UartBufferedTx};

pub struct UartBufferedRxAsync {
    uart: UartBufferedRx,
}

impl UartBufferedRxAsync {
    pub fn new(uart: UartBufferedRx) -> Self {
        UartBufferedRxAsync { uart }
    }
}

impl AsyncRead for UartBufferedRxAsync {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        let s = self.get_mut();
        let uart = &mut s.uart;

        if let Some(len) = uart.read_nb(buf) {
            return Poll::Ready(Ok(len));
        }

        // Need to register for readiness on the signal. We wait to clear the
        // signal until after the uart is not ready so that we don't make a
        // redundant system call before each read attempt, e.g. if the client is
        // reading one byte at a time and there are several buffered.
        // Because the signal is edge triggered, resetting here could clear an
        // event that happened since the poll above. So poll one more time.
        let signal = uart.get_signal();
        signal.reset::<C>();
        current_reactor_register(signal, cx);

        if let Some(len) = uart.read_nb(buf) {
            return Poll::Ready(Ok(len));
        }

        Poll::Pending
    }

    unsafe fn initializer(&self) -> Initializer {
        Initializer::nop()
    }
}

pub struct UartBufferedTxAsync {
    uart: UartBufferedTx,
}

impl UartBufferedTxAsync {
    pub fn new(uart: UartBufferedTx) -> Self {
        UartBufferedTxAsync { uart }
    }
}

impl AsyncWrite for UartBufferedTxAsync {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        let s = self.get_mut();
        let uart = &mut s.uart;

        if let Some(len) = uart.write_nb(buf) {
            return Poll::Ready(Ok(len));
        }

        // Need to register for readiness on the signal. We wait to clear the
        // signal until after the uart is not ready so that we don't make a
        // redundant system call before each write attempt, e.g. if the client
        // is writing one byte at a time and there are several buffered.
        // Because the signal is edge triggered, resetting here could clear an
        // event that happened since the poll above. So poll one more time.
        let signal = uart.get_signal();
        signal.reset::<C>();
        current_reactor_register(signal, cx);

        if let Some(len) = uart.write_nb(buf) {
            return Poll::Ready(Ok(len));
        }

        Poll::Pending
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}
