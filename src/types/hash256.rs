use std::{
    cmp::PartialEq,
    convert::{AsRef, From},
    default::Default
};

//warning LE on wire, keeping format!
#[derive(PartialEq, Copy, Clone)]
pub struct Hash256(pub [u8; 32]);

impl Hash256 {
    pub fn new(slice: &[u8]) -> Hash256 {
        let mut arr = [0; 32];
        arr.copy_from_slice(slice);
        Hash256(arr)
    }
    pub fn is_zero(&self) -> bool {
        let Hash256(hash) = self;
        let zeros = &[0u8; 32][..];
        if hash == zeros {
            return true;
        }
        false
    }
}

//we print the hash in BE, as that is how bitcoind and block explorers show it
impl std::fmt::Debug for Hash256 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let Hash256(hash) = self;
        for byte in hash.iter().rev() {
            write!(f, "{:02X}", byte)?
        }
        write!(f, "")
    }
}

impl Default for Hash256 {
    fn default() -> Hash256 {
        Hash256::new(&[0u8; 32][..])
    }
}

impl AsRef<[u8]> for Hash256 {
    fn as_ref(&self) -> &[u8] {
        let Hash256(hash) = self;
        &hash[..]
    }
}

impl From<&[u8]> for Hash256 {
    fn from(slice: &[u8]) -> Hash256 {
        Hash256::new(slice)
    }
}
