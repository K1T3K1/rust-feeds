pub fn read_str_with_len(len_str_slice: &[u8]) -> Result<(usize, &str), std::io::Error> {
    let str_len = u8::from_be_bytes([len_str_slice[0]]);
    if len_str_slice.len() < 1 + str_len as usize {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Data too short",
        ));
    }

    let str_text = &len_str_slice[0..str_len as usize];
    match std::str::from_utf8(str_text) {
        Ok(on) => return Ok((str_len as usize, on)),
        Err(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Str not UTF-8",
            ))
        }
    }
}
