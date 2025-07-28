use crate::error::UnixSockApiError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FramingError {
    #[error("IO error during framing: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Message too large: {0} bytes")]
    MessageTooLarge(usize),
    
    #[error("Invalid frame format: {0}")]
    InvalidFrame(String),
}

impl From<FramingError> for UnixSockApiError {
    fn from(error: FramingError) -> Self {
        match error {
            FramingError::IoError(io_err) => UnixSockApiError::IoError(io_err.to_string()),
            FramingError::MessageTooLarge(size) => UnixSockApiError::MessageTooLarge(size, 10_000_000),
            FramingError::InvalidFrame(msg) => UnixSockApiError::MalformedData(msg),
        }
    }
}

/// Message framing with 4-byte big-endian length prefix (exact Swift implementation)
pub struct MessageFrame;

impl MessageFrame {
    const MAX_FRAME_SIZE: usize = 100_000_000; // 100MB absolute maximum
    
    /// Write a framed message to an async writer
    pub async fn write_frame<W>(writer: &mut W, message: &[u8]) -> Result<(), FramingError>
    where
        W: AsyncWriteExt + Unpin,
    {
        let message_len = message.len();
        
        // Check message size limit
        if message_len > Self::MAX_FRAME_SIZE {
            return Err(FramingError::MessageTooLarge(message_len));
        }
        
        // Write 4-byte length prefix (big-endian)
        let length_bytes = (message_len as u32).to_be_bytes();
        writer.write_all(&length_bytes).await?;
        
        // Write message payload
        writer.write_all(message).await?;
        
        // Ensure data is written
        writer.flush().await?;
        
        Ok(())
    }
    
    /// Read a framed message from an async reader
    pub async fn read_frame<R>(reader: &mut R) -> Result<Vec<u8>, FramingError>
    where
        R: AsyncReadExt + Unpin,
    {
        // Read 4-byte length prefix
        let mut length_bytes = [0u8; 4];
        reader.read_exact(&mut length_bytes).await?;
        let message_len = u32::from_be_bytes(length_bytes) as usize;
        
        // Validate message length
        if message_len == 0 {
            return Err(FramingError::InvalidFrame("Message length cannot be zero".to_string()));
        }
        
        if message_len > Self::MAX_FRAME_SIZE {
            return Err(FramingError::MessageTooLarge(message_len));
        }
        
        // Read message payload
        let mut message_buffer = vec![0u8; message_len];
        reader.read_exact(&mut message_buffer).await?;
        
        Ok(message_buffer)
    }
    
    /// Calculate total frame size (length prefix + payload)
    pub fn frame_size(payload_size: usize) -> usize {
        4 + payload_size // 4 bytes for length prefix + payload
    }
    
    /// Validate frame size against limits
    pub fn validate_frame_size(payload_size: usize, max_size: usize) -> Result<(), FramingError> {
        if payload_size > max_size {
            return Err(FramingError::MessageTooLarge(payload_size));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    
    #[tokio::test]
    async fn test_frame_write_read() {
        let test_message = b"Hello, World!";
        let mut buffer = Vec::new();
        
        // Write frame
        MessageFrame::write_frame(&mut buffer, test_message).await.unwrap();
        
        // Read frame back
        let mut cursor = Cursor::new(buffer);
        let read_message = MessageFrame::read_frame(&mut cursor).await.unwrap();
        
        assert_eq!(test_message, read_message.as_slice());
    }
    
    #[tokio::test]
    async fn test_empty_message_error() {
        let mut buffer = Vec::new();
        
        // Write empty message
        MessageFrame::write_frame(&mut buffer, &[]).await.unwrap();
        
        // Try to read frame back
        let mut cursor = Cursor::new(buffer);
        let result = MessageFrame::read_frame(&mut cursor).await;
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FramingError::InvalidFrame(_)));
    }
    
    #[tokio::test]
    async fn test_large_message_error() {
        let large_message = vec![0u8; MessageFrame::MAX_FRAME_SIZE + 1];
        let mut buffer = Vec::new();
        
        let result = MessageFrame::write_frame(&mut buffer, &large_message).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FramingError::MessageTooLarge(_)));
    }
    
    #[test]
    fn test_frame_size_calculation() {
        assert_eq!(MessageFrame::frame_size(100), 104); // 4 + 100
        assert_eq!(MessageFrame::frame_size(0), 4);     // 4 + 0
    }
}