#[cfg(feature = "http")]
mod http;

#[test]
fn constructing_path_gives_expected_string() {
    use super::extension_trait::make_path_from_prefix_and_address;

    const ADDRESS: [u8; 20] = hex_literal::hex!("1c0c490f1b5528d8173c5de46d131160e4b2c0c3");
    const PREFIX: &[u8] = b"a/path/to/an/address/";
    let expected = "a/path/to/an/address/1c0c490f1b5528d8173c5de46d131160e4b2c0c3";
    let actual = make_path_from_prefix_and_address(PREFIX, ADDRESS);
    assert_eq!(expected, actual);
}
