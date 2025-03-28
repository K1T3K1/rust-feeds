use std::sync::Arc;

use textnonce::TextNonce;

use crate::{
    message_string::read_str_with_len,
    server::{BROKER_NAME, NAME_LENGTH, SUBS},
};
use smol::{
    future,
    io::{AsyncReadExt, AsyncWriteExt},
    lock::Mutex,
    net::TcpStream,
};

#[repr(u8)]
enum OpCodes {
    ErrorCode,
    Info,
    Auth,
    Publish,
    Subscribe,
    Unsubscribe,
}

impl TryFrom<u8> for OpCodes {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(Self::ErrorCode),
            1 => Ok(Self::Info),
            2 => Ok(Self::Auth),
            3 => Ok(Self::Publish),
            4 => Ok(Self::Subscribe),
            5 => Ok(Self::Unsubscribe),
            _ => Err(()),
        }
    }
}
#[inline(always)]
pub async fn write_info_message(stream: &mut TcpStream) -> Result<TextNonce, std::io::Error> {
    let nonce = TextNonce::new();
    let total_len = 6 + 32 + NAME_LENGTH as usize;
    let total_len_32 = total_len as u32;
    let mut data: Vec<u8> = Vec::with_capacity(total_len);
    data.extend_from_slice(&total_len_32.to_be_bytes());
    data.push(OpCodes::Info as u8);
    data.push(NAME_LENGTH);
    data.extend_from_slice(&BROKER_NAME.as_bytes());
    data.extend_from_slice(&nonce.as_bytes());

    stream.write_all(&data).await?;

    return Ok(nonce);
}

#[inline(always)]
pub async fn read_auth_message(stream: &mut TcpStream) -> Result<Vec<u8>, std::io::Error> {
    let mut auth_buf = [0u8; 4];
    stream.read_exact(&mut auth_buf).await?;
    let len = u32::from_be_bytes(auth_buf);

    if len < 39 {
        write_error_message(
            stream,
            &format!("Expected at least 34 bytes. Got: {}.", auth_buf.len()),
        )
        .await?;
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Data length less than excepted",
        ));
    }

    let mut data_buf = vec![0u8; (len - 4) as usize];
    stream.read_exact(&mut data_buf).await?;

    if data_buf[0] != OpCodes::Auth as u8 {
        write_error_message(
            stream,
            &format!("Invalid Error Code. Expected 2. Got: {}.", auth_buf[4]),
        )
        .await?;
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Wrong op code provided",
        ));
    }

    let mut data = Vec::with_capacity(len as usize);
    data.extend_from_slice(&auth_buf);
    data.extend_from_slice(&data_buf);

    return Ok(data);
}

#[inline(always)]
pub async fn read_arbitrary_message(
    stream_reader: &mut TcpStream,
    stream_writer: &Arc<Mutex<TcpStream>>,
    read_length: u32,
) -> Result<(), std::io::Error> {
    let mut buff = Vec::with_capacity(read_length as usize);
    buff.extend_from_slice(&read_length.to_be_bytes());
    stream_reader.read_exact(&mut buff).await?;

    match buff[4].try_into() {
        Ok(OpCodes::ErrorCode) => {
            let mut sw = stream_writer.lock().await;
            wrong_op_code_response(&mut sw, OpCodes::ErrorCode).await?
        }
        Ok(OpCodes::Info) => {
            let mut sw = stream_writer.lock().await;
            wrong_op_code_response(&mut sw, OpCodes::Info).await?
        }
        Ok(OpCodes::Auth) => {
            let mut sw = stream_writer.lock().await;
            wrong_op_code_response(&mut sw, OpCodes::Auth).await?
        }
        Ok(OpCodes::Publish) => {
            let mut sw = stream_writer.lock().await;
            publish_message(&mut sw, &buff).await?
        }
        Ok(OpCodes::Subscribe) => todo!(),
        Ok(OpCodes::Unsubscribe) => todo!(),
        Err(_) => todo!(),
    }

    Ok(())
}

#[inline(always)]
async fn publish_message(stream_writer: &mut TcpStream, data: &[u8]) -> Result<(), std::io::Error> {
    if data.len() < 6 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Data too short",
        ));
    }

    let (name_len, _) = read_str_with_len(&data[4..])?;
    let (channel_name_len, channel_name) = read_str_with_len(&data[4 + name_len..])?;

    push_publish_data_to_streams(channel_name, &data[4 + name_len + 1 + channel_name_len..])
        .await?;

    Ok(())
}

#[inline(always)]
async fn push_publish_data_to_streams(channel: &str, data: &[u8]) -> Result<(), std::io::Error> {
    let subs_map = SUBS.read().await;
    if let Some(subs_vec) = subs_map.get(channel) {
        let fut = subs_vec.iter().map(|stream_mutex| async {
            let mut stream = stream_mutex.lock().await;
            stream.write_all(data).await
        });
        futures::future::join_all(fut).await;
    }
    Ok(())
}

async fn wrong_op_code_response(
    stream_writer: &mut TcpStream,
    op_code: OpCodes,
) -> Result<(), std::io::Error> {
    write_error_message(
        stream_writer,
        &format!(
            "Client cannot send {} code to server at this point",
            op_code as u8
        ),
    )
    .await?;
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "Wrong op code provided",
    ))
}

async fn write_error_message(
    stream: &mut TcpStream,
    error_message: &str,
) -> Result<(), std::io::Error> {
    let capacity = 5 + error_message.len();
    let mut data: Vec<u8> = Vec::with_capacity(capacity);
    data.extend_from_slice(&capacity.to_be_bytes());
    data.push(OpCodes::ErrorCode as u8);
    data.extend_from_slice(&error_message.as_bytes());

    stream.write_all(&data).await?;
    Ok(())
}
