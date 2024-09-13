#![cfg(all(feature = "j1939", feature = "vcan_tests", feature = "tokio"))]

use serial_test::serial;
use socketcan_j1939::{tokio::AsyncJ1939Socket, Addr, J1939SockAddr, Name, Pgn, SocketOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
#[serial]
async fn echo_test_with_send_to_and_recv_from() {
    let src = J1939SockAddr::from_parts_with_ifname(
        "vcan0",
        Name::NO_NAME,
        Pgn::from(0x2100),
        Addr::from(0x43),
    )
    .expect("Failed to create src addr");
    let src_socket = AsyncJ1939Socket::open(&src).expect("Failed to open sender socket");

    let dst = J1939SockAddr::from_parts_with_ifname(
        "vcan0",
        Name::NO_NAME,
        Pgn::from(0x2100),
        Addr::from(0x47),
    )
    .expect("Failed to create src addr");
    let dst_socket = AsyncJ1939Socket::open(&dst).expect("Failed to open sender socket");

    let send_buf = Name::from(0x9704033501000004).to_le_bytes();
    let bytes_written = src_socket
        .send_to(&dst, &send_buf)
        .await
        .expect("Failed to send to socket");
    assert_eq!(bytes_written, send_buf.len());

    let mut recv_buf = [0u8; 1024];
    let (size, origin) = dst_socket
        .recv_from(&mut recv_buf)
        .await
        .expect("Failed to recv from socket");
    let recv2_buf = &recv_buf[..size];

    assert_eq!(src, origin);
    assert_eq!(recv2_buf.len(), send_buf.len());
    assert_eq!(recv2_buf, send_buf);
}

#[tokio::test]
#[serial]
async fn test_send_address_claim() {
    let addr = J1939SockAddr::from_parts_with_ifname(
        "vcan0",
        Name::from(0x9704033501000004),
        Pgn::NO_PGN,
        Addr::from(0x40),
    )
    .expect("Failed to create src addr");
    let mut socket = AsyncJ1939Socket::open(&addr).expect("Failed to open sender socket");
    socket
        .set_broadcast(true)
        .expect("Failed to set broadcast option");
    let dest = J1939SockAddr::from_parts_with_ifname(
        "vcan0",
        Name::NO_NAME,
        Pgn::from(0x0EE00),
        Addr::NO_ADDR,
    )
    .expect("Failed to create src addr");

    let send_buf = addr.name().to_le_bytes(); // Not sure if this is the correct order
    let bytes_written = socket
        .send_to(&dest, &send_buf)
        .await
        .expect("Failed to send to socket");

    assert_eq!(bytes_written, send_buf.len());
}

#[tokio::test]
#[serial]
async fn echo_test_with_address_claim() {
    let addr = J1939SockAddr::from_parts_with_ifname(
        "vcan0",
        Name::from(0x9704033501000004),
        Pgn::from(0x2100),
        Addr::from(0x40),
    )
    .expect("Failed to create src addr");
    let mut socket = AsyncJ1939Socket::open(&addr).expect("Failed to open sender socket");
    socket
        .set_broadcast(true)
        .expect("Failed to set broadcast option");

    let other_addr = J1939SockAddr::from_parts_with_ifname(
        "vcan0",
        Name::from(0x9704033501000003),
        Pgn::from(0x2000),
        Addr::from(0x41),
    )
    .expect("Failed to create src addr");
    let mut other_socket =
        AsyncJ1939Socket::open(&other_addr).expect("Failed to open sender socket");
    other_socket
        .set_broadcast(true)
        .expect("Failed to set broadcast option");

    let ac_addr = J1939SockAddr::from_parts_with_ifname(
        "vcan0",
        Name::NO_NAME,
        Pgn::from(0x0EE00),
        Addr::NO_ADDR,
    )
    .expect("Failed to create src addr");
    let send_buf = addr.name().to_le_bytes();
    let bytes_written = socket
        .send_to(&ac_addr, &send_buf)
        .await
        .expect("Failed to send to socket");
    assert_eq!(bytes_written, send_buf.len());

    let send_buf = other_addr.name().to_le_bytes();
    let bytes_written = other_socket
        .send_to(&ac_addr, &send_buf)
        .await
        .expect("Failed to send to socket");
    assert_eq!(bytes_written, send_buf.len());

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let pgn_request = J1939SockAddr::from_parts_with_ifname(
        "vcan0",
        Name::NO_NAME,
        Pgn::from(0x3200),
        Addr::from(0x42),
    )
    .expect("Failed to create src addr");
    let mut send_buf: [u8; 3] = [0, 0, 0];
    send_buf.clone_from_slice(&Pgn::ADDRESS_CLAIMED.to_le_bytes()); // Not sure if this is the correct order
    let bytes_written = other_socket
        .send_to(&pgn_request, &send_buf)
        .await
        .expect("Failed to send to socket");
    assert_eq!(bytes_written, send_buf.len());

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}

#[tokio::test]
#[serial]
async fn echo_test_with_write_and_read() {
    let src_addr = J1939SockAddr::from_parts_with_ifname(
        "vcan0",
        Name::NO_NAME,
        Pgn::from(0x2100),
        Addr::from(0x43),
    )
    .expect("Failed to create src addr");
    let dest_addr = J1939SockAddr::from_parts_with_ifname(
        "vcan0",
        Name::NO_NAME,
        Pgn::from(0x2000),
        Addr::from(0x47),
    )
    .expect("Failed to create dest addr");

    let mut sender_socket = AsyncJ1939Socket::open(&src_addr)
        .expect("Failed to open sender socket")
        .connect(&dest_addr)
        .expect("Failed to connect sender socket");

    let mut receiver_socket = AsyncJ1939Socket::open(&dest_addr)
        .expect("Failed to open receiver socket")
        .connect(&src_addr)
        .expect("Faile to connect receiver socket");

    let send_buf = [0xFFu8; 3];
    let bytes_written = sender_socket
        .write(&send_buf)
        .await
        .expect("Failed to write to sender socket");
    assert_eq!(bytes_written, send_buf.len());

    let mut recv_buf = [0u8; 10];
    let bytes_read = receiver_socket
        .read(&mut recv_buf)
        .await
        .expect("Failed to read from receiver socket");
    assert_eq!(bytes_read, send_buf.len());

    assert_eq!(&send_buf[..bytes_read], &recv_buf[..bytes_read]);
}

#[tokio::test]
#[serial]
async fn echo_test_with_write_and_read_transport_protocol() {
    let src_addr = J1939SockAddr::from_parts_with_ifname(
        "vcan0",
        Name::NO_NAME,
        Pgn::from(0x2100),
        Addr::from(0x43),
    )
    .expect("Failed to create src addr");
    let dest_addr = J1939SockAddr::from_parts_with_ifname(
        "vcan0",
        Name::NO_NAME,
        Pgn::from(0x2000),
        Addr::from(0x47),
    )
    .expect("Failed to create dest addr");

    let mut sender_socket = AsyncJ1939Socket::open(&src_addr)
        .expect("Failed to open sender socket")
        .connect(&dest_addr)
        .expect("Failed to connect sender socket");

    let mut receiver_socket = AsyncJ1939Socket::open(&dest_addr)
        .expect("Failed to open receiver socket")
        .connect(&src_addr)
        .expect("Faile to connect receiver socket");

    let send_buf = [0x23u8; 50];
    let bytes_written = sender_socket
        .write(&send_buf)
        .await
        .expect("Failed to write to sender socket");
    assert_eq!(bytes_written, send_buf.len());

    let mut recv_buf = [0u8; 60];
    let bytes_read = receiver_socket
        .read(&mut recv_buf)
        .await
        .expect("Failed to read from receiver socket");
    assert_eq!(bytes_read, send_buf.len());

    assert_eq!(&send_buf[..bytes_read], &recv_buf[..bytes_read]);
}
