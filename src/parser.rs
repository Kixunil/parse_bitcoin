use nom::IResult;
use nom::bytes::complete::{take, tag};
use nom::number::complete::{le_u32,le_u16, le_u64, le_u8};
// use nom::combinator::peek;
// use nom::multi::many_till;
use nom::sequence::tuple;
use chrono::prelude::*;
use std::convert::TryInto;
use std::str::FromStr;

pub fn parse_magic_number(input: &[u8]) -> IResult<&[u8], &str> {
    let (i, o) = le_u32(input)?;
    let result = match o {
        0xD9B4BEF9 => "mainnet",
        0xDAB5BFFA => "regtest",
        0x0709110B => "testnet",
        0xFEB4BEF9 => "namecoin",
        _ => "Unknown"
    };
    Ok((i, result))
}

//without header
pub fn parse_block_size (input: &[u8]) -> IResult<&[u8], u32> {
    let (i, size) = le_u32(input)?;
    Ok((i, size))
}

//warning LE on wire, keeping format!
// #[derive(Debug)]
pub struct Hash256([u8;32]);


impl Hash256 {
    fn new(slice: &[u8]) -> Hash256 {
        let mut arr = [0;32];
        arr.copy_from_slice(slice);
        Hash256(arr)
    }
    fn is_zero(&self) -> bool {
        let Hash256(hash) = self;
        let zeros = &[0u8;32][..];
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

#[derive(PartialEq)]
struct Bytes(Vec<u8>);

impl Bytes {
    fn new(slice: &[u8]) -> Bytes{
        Bytes(Vec::from(slice))
    }
    fn len(&self) -> usize {
        let Bytes(bytes) = self;
        bytes.len()
    }
    fn take(&self, n: usize) -> &[u8] {
        let Bytes(bytes) = self;
        &bytes[0..n]
    }
}

impl std::fmt::Debug for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Bytes(bytes) = self;
        // for byte in bytes.iter().rev() {
        for byte in bytes.iter() {
            write!(f, "{:02X}", byte)?
        }
        write!(f, "")
    }
}


#[derive(Debug)]
pub struct BlockHeader {
    version: u32,
    prev_block_hash: Hash256,
    merkle_root_hash: Hash256,
    time: String,
    bits: Bytes,
    nonce: Bytes
}

 impl BlockHeader {
    pub fn new(v: u32, pbh: &[u8], mrh: &[u8], t: u32, b: &[u8], n:&[u8] ) -> BlockHeader{
        BlockHeader{
            version: v,
            prev_block_hash: Hash256::new(pbh),
            merkle_root_hash: Hash256::new(mrh),
            time: chrono::Utc.timestamp(t.try_into().unwrap(), 0u32).to_rfc2822(),
            bits: Bytes::new(b),
            nonce: Bytes::new(n)
        }
    }
}

pub fn parse_block_header(input: &[u8]) -> IResult<&[u8], BlockHeader> {
    let (i, (
        version,
        prev_block_hash,
        merkle_root_hash,
        time,
        bits,
        nonce
    )) = tuple((
        le_u32, //version
        take(32 as usize), //prev_block_hash
        take(32 as usize), //merkle_root_hash
        le_u32, //time
        take(4 as usize), //bits
        take(4 as usize) //nonce
    )
    )(input)?;
    Ok((i, BlockHeader::new(
        version,
        prev_block_hash,
        merkle_root_hash,
        time,
        bits,
        nonce
    )))
}

pub fn parse_var_int(input: &[u8]) -> IResult<&[u8], u64> {
    let (i, o) = le_u8(input)?;
    if o == 0xFD {
        let (i, o) = le_u16(i)?;
        return Ok((i, o.into()))
    }else if o == 0xFE {
        let (i, o) = le_u32(i)?;
        return Ok((i, o.into()))
    }else if o == 0xFF {
        let (i, o) = le_u64(i)?;
        return Ok((i, o.into()))
    }
    Ok((i,o.into()))
}

#[derive(Debug)]
pub struct TxInput {
    previos_tx_hash: Hash256,
    vout: u32,
    script_sig: Bytes,
    // sequence: Bytes
    sequence: u32
}

impl TxInput {
    fn new (ptx: &[u8], vout: u32, scr: &[u8], seq: u32) -> TxInput {
        TxInput {
            previos_tx_hash: Hash256::new(ptx),
            vout,
            script_sig: Bytes::new(scr),
            // sequence: Bytes::new(seq)
            sequence: seq
        }
    }
}

fn parse_tx_inputs(input: &[u8]) -> IResult<&[u8], Vec<TxInput>> {
    let (mut input, in_count) = parse_var_int(input)?;
    let mut vec: Vec<TxInput> = Vec::with_capacity(in_count as usize);
    for _ in 0..in_count {
        let (i, previos_tx_hash) = take(32u32)(input)?;
        let (i, vout) = le_u32(i)?;
        let (i, script_len) = parse_var_int(i)?;
        let (i, script_sig) = take(script_len)(i)?;
        // println!("script_sig: {}", String::from_utf8_lossy(script_sig));
        // let (i, sequence) = take(4u32)(i)?;
        let (i, sequence) = le_u32(i)?;
        input = i;
        vec.push(TxInput::new(
            previos_tx_hash,
            vout,
            script_sig,
            sequence
        ));
    }
    Ok((input, vec))
}

#[derive(Debug)]
pub struct TxOutput {
    value: u64,
    script_pub_key: Bytes
}

impl TxOutput {
    fn new(value: u64, spk: &[u8]) -> TxOutput {
        TxOutput {
            value,
            script_pub_key: Bytes::new(spk)
        }
    }
}

fn parse_tx_outputs(input: &[u8]) -> IResult<&[u8], Vec<TxOutput>> {
    let (mut input, out_count) = parse_var_int(input)?;
    let mut vec: Vec<TxOutput> = Vec::with_capacity(out_count as usize);
    for _ in 0..out_count {
        let (i, value) = le_u64(input)?;
        let (i, script_len) = parse_var_int(i)?;
        let (i, script_pub_key) = take(script_len)(i)?;
        input = i;
        vec.push(TxOutput::new(
            value,
            script_pub_key
        ));
    }
    Ok((input, vec))
}

#[derive(Debug,PartialEq)]
pub struct Witness(Option<Bytes>);

impl Witness {
    fn new(slice: &[u8]) -> Witness {
        let witness = match slice.len() {
            0 => None,
            _ => Some(Bytes::new(slice))
        };
        Witness(witness)
    }
    fn empty() -> Witness {
        Witness(None)
    }
}

fn parse_witnesses(input: &[u8]) -> IResult<&[u8], Vec<Witness>> {
    let mut vec = Vec::new();
    let (mut input, witness_count) = parse_var_int(input)?;
    if witness_count == 0 {
        vec.push(Witness::empty());
    }
    else {
        for _ in 0..witness_count {
            let (i, witness_len) = parse_var_int(input)?;
            let (i, witness) = take(witness_len)(i)?;
            vec.push(Witness::new(witness));
            input = i;
        }
    }
    Ok((input, vec))
}

#[derive(Debug)]
pub struct Transaction {
    version: u32,
    inputs: Vec<TxInput>,
    outputs: Vec<TxOutput>,
    witnesses: Option<Vec<Vec<Witness>>>,
    lock_time: u32
}

impl Transaction {
    fn new (version: u32, inputs: Vec<TxInput>,
            outputs: Vec<TxOutput>, witnesses: Option<Vec<Vec<Witness>>>,
            lock_time: u32) -> Transaction {
        Transaction {
            version,
            inputs,
            outputs,
            witnesses,
            lock_time
        }
    }
}

pub fn parse_transaction (input: &[u8]) -> IResult<&[u8], Transaction> {
    let (input,version) = le_u32(input)?;
    let res : IResult<&[u8], &[u8]> = tag([0x00,0x01])(input);
    let (input, witness_data) = match res {
        Ok((input, _)) => (input, true),
        Err(_) => (input, false)
    };
    let (input, inputs) = parse_tx_inputs(input)?;
    let is_coinbase = inputs[0].previos_tx_hash.is_zero();
    let (mut input, outputs) = parse_tx_outputs(input)?;
    //we count the nunmber of witness inputs
    // let mut inputs_with_witness_count = 0;
    // if is_coinbase && witness_data {
    //     inputs_with_witness_count = 1;
    // }
    // //version 1 txes do not have a witness for all inputs, need to count them
    // else if witness_data && version == 1 {
    //     for input in &inputs{
    //         //empty sciptSig means a native segwit P2WPKH
    //         if input.script_sig.len() == 0 {
    //             inputs_with_witness_count += 1;
    //         }
    //         //160014 in scriptsig means P2SH(P2WPKH)
    //         else if input.script_sig.take(3) == [0x16,0x00,0x14] {
    //                 inputs_with_witness_count += 1;
    //         }
    //         //P2WSH nested in BIP16 P2SH
    //         else if input.script_sig.take(3) == [0x22,0x00,0x20] {
    //                 inputs_with_witness_count += 1;
    //         }
    //     }
    // }
    // else if witness_data && version == 2 {
    //     inputs_with_witness_count = inputs.len();
    // }
    // println!("there are {} inputs with witness", inputs_with_witness_count);

    let witnesses = if witness_data == true {
        // let mut vec = Vec::with_capacity(inputs_with_witness_count);
        let mut vec = Vec::with_capacity(inputs.len());
        for _ in 0..inputs.len() {
            let (i, witnesses) = parse_witnesses(input)?;
            input = i;
            vec.push(witnesses);
        }
        Some(vec)
    } else {
        None
    };
    let (input, lock_time) = le_u32(input)?;
    Ok((input,Transaction::new(
        version,
        inputs,
        outputs,
        witnesses,
        lock_time
    )))
}

#[derive(Debug)]
pub struct Block {
    header: BlockHeader,
    chain: String,
    size: u32,
    transactions: Vec<Transaction>
}

impl Block {
    fn new(h: BlockHeader, c: &str, s: u32, t: Vec<Transaction>) -> Block {
        Block {
            header: h, chain: String::from_str(c).unwrap(), size: s, transactions: t
        }
    }
}

pub fn find_block() {

}

pub fn parse_block(input: &[u8]) -> IResult<& [u8], Block> {
    // let (input, _) = many_till(take(1usize), peek(parse_magic_number))(input).unwrap();
    // let (input, chain) = parse_magic_number(input)?;
    // let (input, size) = le_u32(input)?;
    let chain="mainnet";
    let size=1172657;
    let (input, header) = parse_block_header(input)?;
    println!("header: {:?}", header);
    let (mut input, tx_count) = parse_var_int(input)?;
    println!("tx_count: {:?}", tx_count);
    let mut txs = Vec::with_capacity(tx_count as usize);
    for n in 0..tx_count {
        let (i, tx) = parse_transaction(input)?;
        println!("\rparsed transaction index: {}",n);
        println!("tx: {:?}", tx);
        txs.push(tx);
        input = i;
    }
    Ok((input, Block::new(header, chain, size, txs)))
}


#[cfg(test)]
mod tests {
    use super::*;
    use hex;
    //all macros use wire format!!
    //test_input!(input, "hash",vout,"scriptsig","sequence")
    macro_rules! test_input {
        ($input:expr, $hash:expr, $vout:expr, $script_sig:expr, $sequence:expr) => {
            {
                let Hash256(hash) = $input.previos_tx_hash;
                assert_eq!(hex::encode(hash), $hash);
                assert_eq!($input.vout, $vout);
                let Bytes(script_sig) = &$input.script_sig;
                assert_eq!(hex::encode(script_sig), $script_sig);
                assert_eq!($input.sequence, $sequence as u32);
            }
        };
    }
    //test_output(output, value, script_pub_key)
    macro_rules! test_output {
        ($output:expr, $value:expr, $script_pub_key:expr) => {
            {
                assert_eq!($output.value,$value);
                let Bytes(script_pub_key) = &$output.script_pub_key;
                assert_eq!(hex::encode(script_pub_key), $script_pub_key)
            }
        };
    }
    //test_witness(witness, "" | "witness_script";
    macro_rules! test_witness {
        ($witness:expr, $witness_script:expr) => {
            {
                match $witness {
                    Witness(None) => {
                        assert_eq!("", $witness_script);
                    }
                    Witness(Some(Bytes(bytes))) => {
                        assert_eq!(hex::encode(bytes), $witness_script)
                    }
                }
            }
        }
    }
    #[test]
    fn test_parse_var_int() {
        assert_eq!(parse_var_int(&[0xFA][..]), Ok((&[][..],0xFAu64)));
        assert_eq!(parse_var_int(&[0xFA,0xAA][..]), Ok((&[0xAA][..],0xFAu64)));
        assert_eq!(parse_var_int(&[0xFD,0xAA,0xBB][..]), Ok((&[][..],0xBBAAu64)));
        assert_eq!(parse_var_int(&[0xFD,0xAA,0xBB, 0xCC][..]), Ok((&[0xCC][..],0xBBAAu64)));
        assert_eq!(parse_var_int(&[0xFE,0xAA,0xBB, 0xCC, 0xDD][..]), Ok((&[][..],0xDDCCBBAAu64)));
        assert_eq!(parse_var_int(&[0xFE,0xAA,0xBB, 0xCC, 0xDD, 0xEE][..]), Ok((&[0xEE][..],0xDDCCBBAAu64)));
        assert_eq!(parse_var_int(&[0xFF,0xAA,0xBB, 0xCC, 0xDD,0xEE, 0xFF,0x10, 0x09][..]), Ok((&[][..],0x0910FFEEDDCCBBAAu64)));
        assert_eq!(parse_var_int(&[0xFF,0xAA,0xBB, 0xCC, 0xDD,0xEE, 0xFF,0x10, 0x09,0x08][..]), Ok((&[0x08][..],0x0910FFEEDDCCBBAAu64)));
    }
    #[test]
    fn test_parse_tx_inputs() {
        let data = include_bytes!("tx_640d0279609c9047ebbffb1d0dcf78cbbe2ae12cadd41a28377e1a259ebf5b89.input.bin");
        let (_, inputs) = parse_tx_inputs(data).unwrap();
        assert_eq!(inputs.len(),5);
        test_input!(
            &inputs[0],
            "18b120842f139d232fa9ae944d38f3657aaa83ee3acb773cdafce39c0095bc65",
            0,
            "220020bcf9f822194145acea0f3235f4107b5bf1a91b6b9f8489f63bf79ec29b360913",
            4294967295
        );
        test_input!(
            &inputs[1],
            "e0d2b92daf4a117bc2ef18cb53fc075588db552e62336ece80384dc4e9b26e94",
            0,
            "4830450221008c89d5443e21c6db957ae6238f642e293c501492ad35ab0dc31d79f7f5e3128c02206e6b33b8eead01a1a0cf4e493432c543eb7000ff9077ebded4d6df0f46ab51dd012103efb03c939c79c5b2609c4e4cf296455a4e40688d8f5e89dcda25088049b252cb",
            4294967295
        );
        test_input!(
            &inputs[2],
            "5a55d746ea6c651e0a9830f1129519fbf2afad9551adf41b345b76c28cf1a79c",
            0,
            "483045022100a37a74bf92e77e80a56838d8d4333111e5dcf7029c0fed82a5f777bd37431b1102202c13c26350215cba09d359cef055170d5629ce28ebbd6ee34c66b4ac2a240c57012102bc454fb76c8fb5517c81853458e0cb42c1136869ab7d62250a39261c5c63c43e",
            4294967295
        );
        test_input!(
            &inputs[3],
            "03d843b16ecaa13a0371286d478073728feeac367888f6f146f58dec36cf3351",
            0,
            "483045022100a152a58ceeaa2a8989bb975e84bf3a68ba740bd31e0dd66d72bad64dac8b39b202201c45aeda6a69e364b72390ed8a28d25b10208f7db23c8b5bb54c7ed6122694c2012103f62f4b41ff70a5b6398c961d4c7bae47942ae37b7e1ed00324375af8d005a336",
            4294967295
        );
        test_input!(
            &inputs[4],
            "6a539477a0d1e2760678751d5a3c8667c72b0287e8ea1d347025cc9a45638de7",
            0,
            "473044022075c22dbd96f00c265d8eef217b9c48692334e6cca0c1a49c760b7e47a6273c8202203b25a16ba1aeb6626e4655fbc782253ba1d2666ccdd72638503c1d055d4eeb40012102e162d3d6f52b56dbf59f35ea977d5683b546105fbc9a638b64262192b9ed2da4",
            4294967295
        );
    }
    #[test]
    fn test_parse_tx_outputs(){
        let data = include_bytes!("tx_640d0279609c9047ebbffb1d0dcf78cbbe2ae12cadd41a28377e1a259ebf5b89.output.bin");
        let (_, outputs) = parse_tx_outputs(data).unwrap();
        assert_eq!(outputs.len(), 2);
        test_output!(outputs[0], 7357023, "a91430897cc6c9d69f6a2c2f1c651d51f22219f1a4f687");
        test_output!(outputs[1], 28734702, "a914fa68aba99b21ce4bba393eacc17305fe12f9021b87");
    }
    #[test]
    fn test_parse_witnesses() {
        let data = include_bytes!("tx_640d0279609c9047ebbffb1d0dcf78cbbe2ae12cadd41a28377e1a259ebf5b89.witnesses.bin");
        let (_, witnesses) = parse_witnesses(data).unwrap();
        assert_eq!(witnesses.len(), 4);
        test_witness!(&witnesses[0], "");
        test_witness!(&witnesses[1], "3045022100aa2570dde15cdcb834e3490b8d10787decf3c0f6c388e949177d3531e99068c9022053a2decd7f5859cd5f2a583c8c12ba621f09721b3bc74a64d362bb9c2d57b27e01");
        test_witness!(&witnesses[2], "304402200da46260a1a6b6e7fe0e23372adcf7e9569c9f27501728a5d61ab4a3c74732b302200790fb7ce382c742b8e23f53c302b19a33cba9d68a83f33974b971511e2c712e01");
        test_witness!(&witnesses[3], "5221026c8f72b9e63db63907115e65d4da86eaae595b22fdc85ec75301bb4adbf203582103806535be3e3920e5eedee92de5714188fd6a784f2bf7b04f87de0b9c3ae1ecdb21024b23bfdce2afcae7e28c42f7f79aa100f22931712c52d7414a526ba494d44a2553ae");
    }
    #[test]
    fn test_parse_transaction() {
        //test generated by:$ for i in $(ls tx_*.rpc);do ./generate_tx_tests.sh $i;done

        let data = include_bytes!("tx_640d0279609c9047ebbffb1d0dcf78cbbe2ae12cadd41a28377e1a259ebf5b89.bin");
        let (_, tx) = parse_transaction(data).unwrap();
        assert_eq!(tx.version, 1);
        assert_eq!(tx.lock_time, 0);
        assert_eq!(tx.inputs.len(), 5);
        assert_eq!(tx.outputs.len(), 2);
        let witnesses = match tx.witnesses {
        	Some(witnesses) => witnesses,
        	None => vec![]
        };
        assert_eq!(witnesses.len(), 5);
        let inputs = tx.inputs;
        test_input!(&inputs[0], "18b120842f139d232fa9ae944d38f3657aaa83ee3acb773cdafce39c0095bc65", 0, "220020bcf9f822194145acea0f3235f4107b5bf1a91b6b9f8489f63bf79ec29b360913", 4294967295);
        test_input!(&inputs[1], "e0d2b92daf4a117bc2ef18cb53fc075588db552e62336ece80384dc4e9b26e94", 0, "4830450221008c89d5443e21c6db957ae6238f642e293c501492ad35ab0dc31d79f7f5e3128c02206e6b33b8eead01a1a0cf4e493432c543eb7000ff9077ebded4d6df0f46ab51dd012103efb03c939c79c5b2609c4e4cf296455a4e40688d8f5e89dcda25088049b252cb", 4294967295);
        test_input!(&inputs[2], "5a55d746ea6c651e0a9830f1129519fbf2afad9551adf41b345b76c28cf1a79c", 0, "483045022100a37a74bf92e77e80a56838d8d4333111e5dcf7029c0fed82a5f777bd37431b1102202c13c26350215cba09d359cef055170d5629ce28ebbd6ee34c66b4ac2a240c57012102bc454fb76c8fb5517c81853458e0cb42c1136869ab7d62250a39261c5c63c43e", 4294967295);
        test_input!(&inputs[3], "03d843b16ecaa13a0371286d478073728feeac367888f6f146f58dec36cf3351", 0, "483045022100a152a58ceeaa2a8989bb975e84bf3a68ba740bd31e0dd66d72bad64dac8b39b202201c45aeda6a69e364b72390ed8a28d25b10208f7db23c8b5bb54c7ed6122694c2012103f62f4b41ff70a5b6398c961d4c7bae47942ae37b7e1ed00324375af8d005a336", 4294967295);
        test_input!(&inputs[4], "6a539477a0d1e2760678751d5a3c8667c72b0287e8ea1d347025cc9a45638de7", 0, "473044022075c22dbd96f00c265d8eef217b9c48692334e6cca0c1a49c760b7e47a6273c8202203b25a16ba1aeb6626e4655fbc782253ba1d2666ccdd72638503c1d055d4eeb40012102e162d3d6f52b56dbf59f35ea977d5683b546105fbc9a638b64262192b9ed2da4", 4294967295);

        let outputs = tx.outputs;
        test_output!(outputs[0], 7357023, "a91430897cc6c9d69f6a2c2f1c651d51f22219f1a4f687");
        test_output!(outputs[1], 28734702, "a914fa68aba99b21ce4bba393eacc17305fe12f9021b87");

        let witnesses_n = &witnesses[0];
        test_witness!(&witnesses_n[0], "");
        test_witness!(&witnesses_n[1], "3045022100aa2570dde15cdcb834e3490b8d10787decf3c0f6c388e949177d3531e99068c9022053a2decd7f5859cd5f2a583c8c12ba621f09721b3bc74a64d362bb9c2d57b27e01");
        test_witness!(&witnesses_n[2], "304402200da46260a1a6b6e7fe0e23372adcf7e9569c9f27501728a5d61ab4a3c74732b302200790fb7ce382c742b8e23f53c302b19a33cba9d68a83f33974b971511e2c712e01");
        test_witness!(&witnesses_n[3], "5221026c8f72b9e63db63907115e65d4da86eaae595b22fdc85ec75301bb4adbf203582103806535be3e3920e5eedee92de5714188fd6a784f2bf7b04f87de0b9c3ae1ecdb21024b23bfdce2afcae7e28c42f7f79aa100f22931712c52d7414a526ba494d44a2553ae");
        let witnesses_n = &witnesses[1];
        test_witness!(&witnesses_n[0], "");
        let witnesses_n = &witnesses[2];
        test_witness!(&witnesses_n[0], "");
        let witnesses_n = &witnesses[3];
        test_witness!(&witnesses_n[0], "");
        let witnesses_n = &witnesses[4];
        test_witness!(&witnesses_n[0], "");

        let data = include_bytes!("tx_827214460f979de7023be7cf82bc11fdf9130fec624b99bb0156f580328110b8.pre_segwit.bin");
        let (_, tx) = parse_transaction(data).unwrap();
        assert_eq!(tx.version, 1);
        assert_eq!(tx.lock_time, 0);
        assert_eq!(tx.inputs.len(), 1);
        assert_eq!(tx.outputs.len(), 2);
        let witnesses = match tx.witnesses {
        	Some(witnesses) => witnesses,
        	None => vec![]
        };
        assert_eq!(witnesses.len(), 0);
        let inputs = tx.inputs;
        test_input!(&inputs[0], "bdc03907d321ac2ab812a2c54a5aaea34d57ec7b6ad10c8b139e08e6d25f47a9", 0, "473044022059ef25a8cb0fb2eb097ad3c8b1bed25cfcfa1f606a1ac47cbd5dc0ff688004ad022034418df4599122e387d62b3ae6c1131894509e460afd34d514405fcb6a25fbf8014104075407496e07c698ec874c70db70f3d01d098fc1756d2e070cb97246492b47e8e2e7fabb8c0703c99ed0e728a5676a4620a79126fa5f2c9aa9474371f5c3b7ab", 4294967295);

        let outputs = tx.outputs;
        test_output!(outputs[0], 194000000, "76a9146be6bb0554c1f482c529d018de7da3b039b5ec1188ac");
        test_output!(outputs[1], 1000000, "76a914e110891bfbb319c04762169d3885203b6eb9a25288ac");


        let data = include_bytes!("tx_982e0cea72b4f599e09f3556d649518608385fcf269e811fa7ed51d7e4f5241c.bin");
        let (_, tx) = parse_transaction(data).unwrap();
        assert_eq!(tx.version, 2);
        assert_eq!(tx.lock_time, 607785);
        assert_eq!(tx.inputs.len(), 1);
        assert_eq!(tx.outputs.len(), 2);
        let witnesses = match tx.witnesses {
        	Some(witnesses) => witnesses,
        	None => vec![]
        };
        assert_eq!(witnesses.len(), 1);
        let inputs = tx.inputs;
        test_input!(&inputs[0], "63974dced933b0885531c6cd16b81832465b3ea4d266437afbce6edf6f1dd0e4", 31, "160014c3447428dee50d786fea673c6a0fc32f665a3da8", 4294967293);

        let outputs = tx.outputs;
        test_output!(outputs[0], 212948850, "76a914d89793719a269a2bb180886a95d3a5d83c3adc8888ac");
        test_output!(outputs[1], 19332281, "a9142c90f403bd58916e85ec98b3c4f9ba7dc0e4e76e87");

        let witnesses_n = &witnesses[0];
        test_witness!(&witnesses_n[0], "304402204ede3bae0185c9c2553936ff10d8daadca99b504ac5bc486a367ef6fa978132a022035398a5365be1a21fc315c0f6d4ddcce059ce876a2ff9119f86a2728d71b5cda01");
        test_witness!(&witnesses_n[1], "02d1c1634a5f48c59ffda1e453a0286fce38514140b63aec09c35c3f4ffa0756bb");

        let data = include_bytes!("tx_9e48f98e0b27e09ccabf576076c01dc6277c3961c8f616dea154f6822fb17765_large_segwit.bin");
        let (_, tx) = parse_transaction(data).unwrap();
        assert_eq!(tx.version, 2);
        assert_eq!(tx.lock_time, 0);
        assert_eq!(tx.inputs.len(), 26);
        assert_eq!(tx.outputs.len(), 10);
        let witnesses = match tx.witnesses {
        	Some(witnesses) => witnesses,
        	None => vec![]
        };
        assert_eq!(witnesses.len(), 26);
        let inputs = tx.inputs;
        test_input!(&inputs[0], "00114b16b5fe1251ee3e995b4dacdd68a815e57978511cda8fd30a79ab414f8b", 0, "16001403fa4c9e95ee6b8863220374db0397b4e52a1a62", 4294967295);
        test_input!(&inputs[1], "e442053bee8f18134cf7d67227ae225e8fd547b1ff7e069d1614763d3836e09a", 25, "160014727bc58035613ba0ef41239626d6921c56796f59", 4294967295);
        test_input!(&inputs[2], "a9f5983dca1b661ad298eab8e414d5d97409145b9b52522767b2ade551cb0772", 56, "160014afa1688ec93089773cd42574c438b852148a75e3", 4294967295);
        test_input!(&inputs[3], "e320afe366dc3f09f3fd5f1310dbf01090f0de07748ccf5c912c93301f23b28a", 0, "1600146f840b4216907270f7b9e02499aea0d342ca5df7", 4294967295);
        test_input!(&inputs[4], "3442ae2122a3c4789e1c7426db81e404e48e413a203bca02334354993b72b2c3", 6, "160014c286b328eae6566dd32559df04c00a73d3e93e56", 4294967295);
        test_input!(&inputs[5], "3f37453fe733f34e33ece620c2c74e803ca83a0ebbe58965082cdf4380ca7f3a", 23, "160014103bd1496525e7871295d0b4414e790a9f34fdd8", 4294967295);
        test_input!(&inputs[6], "9ae8ff52416cfc03e0954372c5bc9a5caaca01c52626176486f736208dda5752", 0, "160014609ebfe943ea2adca3054e018b4bddd213615cf3", 4294967295);
        test_input!(&inputs[7], "c518909f344f0c9f72ccd88bfa43e82bdd68d16e376842f4ffea9f49372db830", 1, "1600144683c3d669e4dc873fc07d95b6360c7eb35d250e", 4294967295);
        test_input!(&inputs[8], "7f9a81b14794c6acd63a7b859754e87059def91e4f417e1841e25fa71f6a5cec", 10, "160014621db2b20233233c9fb97b614df01e9eeacc376b", 4294967295);
        test_input!(&inputs[9], "a8c183e053fd3f57b48346955623b4a56f1a57d19c291512fcbc611c03ce4839", 13, "16001421ad97c14a8ccc7a5f1b48992dd524ffc0a2b4f4", 4294967295);
        test_input!(&inputs[10], "70582a5194b35bb731e462684a3ad6125dfa64a7b54da216f956ba2311c67698", 5, "160014588621703846a8e98861b9c74dab0e959e5dea71", 4294967295);
        test_input!(&inputs[11], "5f2f8e75054e3de9e3d8445174c12db7ec3854720dcc18578ab01b65bfeac618", 17, "160014d88f122ffb37b3c491ba6d338dfe383390578207", 4294967295);
        test_input!(&inputs[12], "4d2f4d68775c3b295ecf2a60fc439cfb17da1ea01f3bc0b92137eb93b2fef6a6", 0, "16001413bba8944bd92eab2dbbd10621b73bc22cffbcbc", 4294967295);
        test_input!(&inputs[13], "6d0c038229f588ed9ee8c25590b3271b4c30a55d8e965bb7a000945e1562446e", 3, "160014fedd82c1a95b2d7c304b3c91f49a2426456f503e", 4294967295);
        test_input!(&inputs[14], "6de9c43ff56ec3b3abb1a4419c6ec447dbbcf8eed11b22ee26d12c87abe2e3b9", 0, "16001468fc848137d7d07df1ac531689d0b0cc20e0ad70", 4294967295);
        test_input!(&inputs[15], "041b1d7b083ea53dad2b7f666a8b839cdbc08b2100e430411e4fcd4500fb91b4", 0, "160014d5f5014f9453c0c30f7a9e68dd1d341c72b925e3", 4294967295);
        test_input!(&inputs[16], "e320afe366dc3f09f3fd5f1310dbf01090f0de07748ccf5c912c93301f23b28a", 6, "1600141e05072012c3d9dee70a0b26c148814acbf9732d", 4294967295);
        test_input!(&inputs[17], "612c171a1fd0a779232609361696a2b5aee21805d39a7c96632c8a9d577823e1", 2, "1600143e3b52fbfa68d340bdcb224e0401a0bde7139c2b", 4294967295);
        test_input!(&inputs[18], "6ae9a523259f51b15c611bf97f60fe721576a4483b31a69a3e34026d2f7f5052", 0, "160014622f8f19af87a47db37bf4f1b1cfdfa6f67e7c9e", 4294967295);
        test_input!(&inputs[19], "34d89e7ad61d975de9c4b263a682ad3e5017de4d4911c4f7353db3721b29f01b", 17, "160014339baaaec1b3967baf81ca597b82e2ffbb47d74b", 4294967295);
        test_input!(&inputs[20], "b5e1a1336f62314e4d23ed1114277c0599ebb81859bb6176b2422513a82c7306", 7, "160014ed1d6db6328af2355f72063e92cd0c0c0e1190ab", 4294967295);
        test_input!(&inputs[21], "f7d27d533eb240a31b5f55568c060195d0fb258951f893ed17ff5722818a66a3", 0, "1600143ef31046d066525a1ddc1ac7f8eb754089aa0dc2", 4294967295);
        test_input!(&inputs[22], "3712edd101cd481ea3bf125f0e08e826dcb682b80150a54f308d347678dcc654", 1, "1600142be2af7e083b4157e4882ffd7763ce4d9530ec2d", 4294967295);
        test_input!(&inputs[23], "33819ccc8bb7cafad4a8a5a4fed576ddd993210c868fc5338c25299c3ca68f98", 3, "160014993ac4ea5060d86fd947bbf99fe29f68c3f05d15", 4294967295);
        test_input!(&inputs[24], "c09ca095b364290a6cb7cb2c3d42c1ae3759d1286226d3edc20f75abcbb987fc", 6, "160014734750f47db08c3df0c96504089e686142d02c6d", 4294967295);
        test_input!(&inputs[25], "1648667c00f8686cec7c6dd2f5064fba8436a5ccf0cd5e6cd5b63069975d16e5", 3, "16001468ffe74657fc33ffdc0a380efacbad637361eb74", 4294967295);

        let outputs = tx.outputs;
        test_output!(outputs[0], 1000000, "a9149eedb19fe75f95ec54543ff73cafaaf0c52c37eb87");
        test_output!(outputs[1], 285799, "76a914b25726349fbe21cc2976efb871efcd7cc444cb0988ac");
        test_output!(outputs[2], 433200, "a91442ad19bcd23b4eae8ec665f7af84b16325167b6487");
        test_output!(outputs[3], 32272348, "76a914d58221209badcf0c62e47b388ce34866a3563d9688ac");
        test_output!(outputs[4], 545000, "76a914c75a85be774e4dfdbbcd755be18c63b5f3e0e4bd88ac");
        test_output!(outputs[5], 100000, "76a9148354f623991b341646e7f2ec0eb2f780c83c2a0788ac");
        test_output!(outputs[6], 3333000, "a9149730cf919cfc249a61bec8af3266622c5da9fa3187");
        test_output!(outputs[7], 15247342, "76a914a7d7f17a4ead33468a20ad5cf26b17a7bc896a0188ac");
        test_output!(outputs[8], 10877580, "a91417ba68e3977d75bf87bcd3d475ed003e5a253d7187");
        test_output!(outputs[9], 1034933, "a9146b7951bf7d165df60f4b92f8bb2eebf3d91369bf87");

        let witnesses_n = &witnesses[0];
        test_witness!(&witnesses_n[0], "3044022001ccc1cb9722c72e8b3b3b4097afe4b4d4eea9588ad53635e3fec4c62465aac002207c1d4706fc7cf05f78277230005dcc2662ff596ca48cc4d60efa65eae38a0ab801");
        test_witness!(&witnesses_n[1], "0343a3c7c5edb72fd20cfcdca0d42b11cd718bd254bff4887fa276a4fde6836c3a");
        let witnesses_n = &witnesses[1];
        test_witness!(&witnesses_n[0], "3044022060249a4d59fc325fbf5ae947c979cbf095e480d55596bc2dbd0bfde38920b7a902205a967363488957ab405274f0e29862a92418bdf69670fce78994233174e6410701");
        test_witness!(&witnesses_n[1], "02d9b90d8dae4a54250616d06ead838a6dd2f8e977efc0118b46b4e1ede1918ba3");
        let witnesses_n = &witnesses[2];
        test_witness!(&witnesses_n[0], "3044022000dbcdf0d7049789e312991412e03109657362768137ab8d6ddeb4253d42f2bb022009730cd86d4394ef79eb0bda70c67147cbb9dd307054e585e8874ca790fab33901");
        test_witness!(&witnesses_n[1], "0396acb01d48f5f5d087efff3d759b0532058f5a87e6a1abd6d5521cad977f401b");
        let witnesses_n = &witnesses[3];
        test_witness!(&witnesses_n[0], "3045022100f234c6822ee34a14849c27ca0a987efdab5b66b2145c528873afac0e6599f0c802204bc1da5eb2d12f936bc41bf02e158f9da639338216ec1ce91c850ab92715aa1d01");
        test_witness!(&witnesses_n[1], "03edf1e2ba356dd932f64d40fb1e459f482e451eadf858a79711d3b23b1054e2fa");
        let witnesses_n = &witnesses[4];
        test_witness!(&witnesses_n[0], "3044022079c50f6bde07c8f40e5e07b9c96dd85520d9a95182ad816f38a80d821d6519e902200d3f073bade7ff9a4bce77b42f34a3ee6ffde41c9a410741b570106fe159fad201");
        test_witness!(&witnesses_n[1], "0331795e2e29c00d5418b6be2289d3a3ad41490e17d1c8158596e1d42f0dbe6cff");
        let witnesses_n = &witnesses[5];
        test_witness!(&witnesses_n[0], "3045022100e07c62e859fad2d7ee5eb04124e61c01168dd66177f1e7960462e84cbdd2b22002200b8565708659c100bcac77197eba38cb98e3f2f6afad762fde1ff1f2578674e701");
        test_witness!(&witnesses_n[1], "0332990ab3fe323c8f0f25c676d794a4428062523a4c951cfe3a107544cb143a88");
        let witnesses_n = &witnesses[6];
        test_witness!(&witnesses_n[0], "3044022002443e52a0179450f83128d7c49cebbc0abdc109df411934171ad96f7b7a387202203cf844d4a776d662452a3db35d6dc20bc09b3eac02c891ce3d6c83e550b870de01");
        test_witness!(&witnesses_n[1], "02edfe3a48594a08a6f310f964a373d0b3dc680478083209f042bb704ce8102576");
        let witnesses_n = &witnesses[7];
        test_witness!(&witnesses_n[0], "304402203c9253ab03d6752073cf3d0e6ffe86fbf467b86a7afd55738da1daba962cdf7e02200d33d658714895a5b45112d1fea9b4dd2258aa89c7e492ddec8dbe6d447ad41001");
        test_witness!(&witnesses_n[1], "02e69d14451661ffd1d6327baf7e8d5b5b1d43012f95334f803444d27e67cd7f0a");
        let witnesses_n = &witnesses[8];
        test_witness!(&witnesses_n[0], "304402206f4fdf2d3e2cffcb129aa5a3d469fd4aecf053d78c37d688d450205118d86845022055085fb0b1e99037d625ea9dba467daeb43c529ecc6ad11d29c18d59500edfc801");
        test_witness!(&witnesses_n[1], "02224c0099aae82953670b9bbdc936ee2a26f7d3b538a1016712fdfae8af6141d7");
        let witnesses_n = &witnesses[9];
        test_witness!(&witnesses_n[0], "304402202bc617648720e81ac6791ab9bc88b482da180fadde05c4d13662d3c9f167e77a022072b56c88921ed2424563fd7db049684811b84bf1b9dc4cefd465ef1e2230140c01");
        test_witness!(&witnesses_n[1], "03759afb6d80611b6777befab845b663414c8a703456f3fbc58d6adbdd42e1a909");
        let witnesses_n = &witnesses[10];
        test_witness!(&witnesses_n[0], "30450221009eadc1c8ac1967d64d6194fa04919bdf4a8c43a8b80b0bb60d74307202cec761022005e7039590f74184317c4b63e67dfa5c2a6296343e934c584a769dec73048e4501");
        test_witness!(&witnesses_n[1], "02a337ce8f564421b93a517328426916ca64d27c2258475c98927d82cfd72db933");
        let witnesses_n = &witnesses[11];
        test_witness!(&witnesses_n[0], "3045022100b0645a14cbc60d4a58daac5c5d37ab799530264e93003d55c2a7d15cd427513c022055e7a052ea8851a92d08ed0e2fecbda48a7478390b5a94afb12cbe439049f61c01");
        test_witness!(&witnesses_n[1], "03dfb2f185226af48a2cd0e44a9d3c198918a117639df87e001d1048365a109af9");
        let witnesses_n = &witnesses[12];
        test_witness!(&witnesses_n[0], "3045022100e54460f6a2751e0c36115226a047e9e959061b849bdb1e4f97a251fa6af15c0a02200d1af6f6c449ebe34c0061828dc223cd924e883f27307da39e9d0b4fabf200d801");
        test_witness!(&witnesses_n[1], "023c2b15c0f755986ec4a34eb1ce4a1ebcab747c34f4bd245b787f3285af166101");
        let witnesses_n = &witnesses[13];
        test_witness!(&witnesses_n[0], "304402200a48cbf5c9d298f8087406699165ed9f0b6594fa0ac2698ca2e30115d8fdaf7402206a0746ead8af976665e0454682e7b7702ecbe572ab3edbe13cc79eff6574915d01");
        test_witness!(&witnesses_n[1], "02933be19b4fec88b0d76116d86bc96a4d0422be0d3873f402a618a9a9c764bde4");
        let witnesses_n = &witnesses[14];
        test_witness!(&witnesses_n[0], "3045022100e50d8d7eca7039cac59d4b925b6d2d92577b6de6e62e48e3b1dc7b246be37c8c022052aff06249bbffa2ebe17f5cc1c03e2006fc1e1b0649298334ac197a9107418901");
        test_witness!(&witnesses_n[1], "03df6a959b7d4f00e1e80cbb2009bfe360e52f9de6166d61e647bd29164f12a633");
        let witnesses_n = &witnesses[15];
        test_witness!(&witnesses_n[0], "3044022004a8bd66049fb0aec610025e54be6c1e9ee885ecda8c25ef8d16e06824e9243802203464d8e2cda94280651399a483df5432fee44d45e806b9395af56462ce4bd1a701");
        test_witness!(&witnesses_n[1], "0379f3b349fe60faed0bfc88ec6786c66fa9acff3b4b14d2b3e8328cda4ded8b38");
        let witnesses_n = &witnesses[16];
        test_witness!(&witnesses_n[0], "3045022100b9d525062cc655c611205d02a040d5959bb8e823bb7894931f23194ed1dd45ea022007b832bd198a8376fc097cedfa3e4452ed78604a8e9a8675b5ece7d3e22fc2a401");
        test_witness!(&witnesses_n[1], "03152c416f1acbe367f78fe9bc2ee931c79a0ca739a56c5b3bca38e00d54c74352");
        let witnesses_n = &witnesses[17];
        test_witness!(&witnesses_n[0], "3045022100ab3b24d535ad21d4903e9f7a17935fc0747544f9b15e035bff3d9e14e7a260e20220460e90f03e3c083286b5460d6fc111ff542664931df86d0c692afe370dbbb7da01");
        test_witness!(&witnesses_n[1], "035299a12a9e259d3fb9eeab3a47ee63cf73d88f2fd52203753e54d1c7d126c722");
        let witnesses_n = &witnesses[18];
        test_witness!(&witnesses_n[0], "304402205c350f9e36362493e9f73f63296add89f6dee7143c7540afba9f9782afdb6ebd02201ae7d732b9c153755bec13e38be0860b7970b2ff2ba44ca916e10e25e1b8837201");
        test_witness!(&witnesses_n[1], "0396351c862b362109637727ee03d6f1dba0fbd205b53a6f389e512fc67cbd96b3");
        let witnesses_n = &witnesses[19];
        test_witness!(&witnesses_n[0], "3044022064d8c09b1ca56ccbab1dbccfd9b8ee1e08136638f827c2950b76e2b04bce168c022058c7e6a8723045842cf386f3bedd3ac6f0f69d8d394e3b9a20a614359477580c01");
        test_witness!(&witnesses_n[1], "032a199f5d5c463fe15287afbb51f96f6bff7b8464f3c16f2be1b02e44a85aad44");
        let witnesses_n = &witnesses[20];
        test_witness!(&witnesses_n[0], "3044022054113bb0264a7c79a248f5525af6692eb2f2130833c22764babf0baf6b776d320220639e6a3aa008f6e6bebe59e8d6dba6abf4931e51e7002b513b6bda7b07364add01");
        test_witness!(&witnesses_n[1], "02ea1e28d10f0795393ba983059171ffed19a892d83433f087073333237ca416b5");
        let witnesses_n = &witnesses[21];
        test_witness!(&witnesses_n[0], "30440220603351681f8dedccfd1bddbcffd12f366a36dad6b36345816e0f31074c1d649b02204da6047a693d2197a83e7126e9fbede495c37a7e6195c06a856afa83ad5a410601");
        test_witness!(&witnesses_n[1], "03420c0d3dabfa6ac17c82f3a66fcd17e9f19e852b2e72721c9a6af96395b87e58");
        let witnesses_n = &witnesses[22];
        test_witness!(&witnesses_n[0], "3044022034cf58fe7c15b8b93be4f06c1b5a162e2e9892fcd4df5eb490b7fbac3933e42502200120be2b5015963602a29ec543b226d57440bdc054b137f4c7acdbfc7260fb3c01");
        test_witness!(&witnesses_n[1], "038daabf8643d7e6728a2c5b28e3a7fe551e2d5e55c8a63b6a6559f82474f677d7");
        let witnesses_n = &witnesses[23];
        test_witness!(&witnesses_n[0], "304402200d09ed7473f108155d3b9a35fd678e7da54d5505272568eeed78ac587ea995f002205027fcd7c8cd170eb5176792e09fcffaedf6a31dcc9b65783cc5b0121dfd0ca301");
        test_witness!(&witnesses_n[1], "0370b1c452105b1818fd328bbb828f95c965f42a626122d52de0483aa6532a0c9e");
        let witnesses_n = &witnesses[24];
        test_witness!(&witnesses_n[0], "304402205b634f9773b1efe70b908f8f1e598a6704bc681aae7c16a2a79c502680df448a0220530dc2bcb6e9d8ceb492332236c4c52933d1574f1cf4606564879776ea3651a201");
        test_witness!(&witnesses_n[1], "02f7e21abca976847a287855394bc49a6395b8c33e95ce08e0715ab7490998d89e");
        let witnesses_n = &witnesses[25];
        test_witness!(&witnesses_n[0], "304402202c941f90c16403b44fbfbfa7746fdf662f8a31629d8bada0ceb767cf3cee92c20220343232ffd0ca52f501719c294f8059964dd57ce9e0383cc4755170fa44edb0da01");
        test_witness!(&witnesses_n[1], "03e1fdf492d6a179219ced7ec684ecccda207b841cd035b31427e7a34012719cb1");

        let data = include_bytes!("tx_c623634f506375a45ee09379d4b117d5ddb1d02eb04c257d9354cbf0055ad191.bin");
        let (_, tx) = parse_transaction(data).unwrap();
        assert_eq!(tx.version, 2);
        assert_eq!(tx.lock_time, 607784);
        assert_eq!(tx.inputs.len(), 4);
        assert_eq!(tx.outputs.len(), 3);
        let witnesses = match tx.witnesses {
        	Some(witnesses) => witnesses,
        	None => vec![]
        };
        assert_eq!(witnesses.len(), 4);
        let inputs = tx.inputs;
        test_input!(&inputs[0], "3882466f2f3e2b3de06354c0fea17b0000ce3fa6ce9698a33c5ec966a7b466a1", 0, "1600144e2f27e5c2e25195634c7713f691f1d482cfdc45", 4294967294);
        test_input!(&inputs[1], "bedb62ebc3dcbef593b3671c583ffc35206e9f9e65ca39a3590383eaae619265", 0, "1600149a67952f08c8ca8b648760cc07fbfa1a554615a6", 4294967294);
        test_input!(&inputs[2], "d55a2c8b450abe65aa025938117b40f6b5db7e7b96673bf3dcc00b31c77e9958", 3, "473044022015270c23c973b6214252739b277e26796f63d70455a336f33b2023d9c4a91c6502203a3bd1ff63835c71dd04ed57b1a8a8a786701ab564fa35e05e2bfc840f6857ef0121023479a5d672f57bc363f24729f4154035813ee0206ef970e08e8a6ecc1c580673", 4294967294);
        test_input!(&inputs[3], "f387b4ecfab822ba6387903e346533586c3e4818ca75f8eec9811a09ae380456", 0, "1600140736c01aa95363f463f747387eec9157884e2de9", 4294967294);

        let outputs = tx.outputs;
        test_output!(outputs[0], 895431, "a914064b4245fa5d570c81812448d131ae52f94b1a1787");
        test_output!(outputs[1], 12680000, "a91484da2a5170dc3d945e7977aed42011d05105505487");
        test_output!(outputs[2], 19170448, "a914938900677f255ebd62d301a96a8470262791136287");

        let witnesses_n = &witnesses[0];
        test_witness!(&witnesses_n[0], "304402203b6c8d47996713857151e1f8e016a452d04db6fdcc5e981255facee21e46178602200fd3fa52621ed5fe6dbb8fbe77e83fbca7d1b326ed410b4a03bac31691ad40ee01");
        test_witness!(&witnesses_n[1], "028b3f127bd412556d78663d8bdbd4277b0510663c7e10778aa4bd91f88db95cfd");
        let witnesses_n = &witnesses[1];
        test_witness!(&witnesses_n[0], "3045022100baae18165ee34f211eb8eaabdb4b40b9dcbc3d2cd472d4741edd236e780b2394022034188b6671061e74c9dd6c61104738db5ed5b2ca1ad947fe545446e938bc338f01");
        test_witness!(&witnesses_n[1], "03d1d0afa87be50294b633a7c7d9e33a523b5003fea45cfda8f715bffc80d66358");
        let witnesses_n = &witnesses[2];
        test_witness!(&witnesses_n[0], "");
        let witnesses_n = &witnesses[3];
        test_witness!(&witnesses_n[0], "3045022100f6fc7a286c18cda3273d49f2742f255be9b28330f295a6e49db9095870acc90002207a99bdd2caf2a9f1698ea64c4be3f21198dfe9a4c3535b25bba6128f36679a6e01");
        test_witness!(&witnesses_n[1], "03a7bcc6fff744df14fd716c712cef7dd6443d9e75851085ec4f33dafcc7772f63");

        let data = include_bytes!("tx_d1425c41b1786b4c7464a9431c2c39bc6920a6d5e6a56295bc0b2e3274941d32.regtest.bin");
        let (_, tx) = parse_transaction(data).unwrap();
        assert_eq!(tx.version, 2);
        assert_eq!(tx.lock_time, 0);
        assert_eq!(tx.inputs.len(), 1);
        assert_eq!(tx.outputs.len(), 2);
        let witnesses = match tx.witnesses {
        	Some(witnesses) => witnesses,
        	None => vec![]
        };
        assert_eq!(witnesses.len(), 1);
        let inputs = tx.inputs;
        test_input!(&inputs[0], "0000000000000000000000000000000000000000000000000000000000000000", 4294967295, "510101", 4294967295);

        let outputs = tx.outputs;
        test_output!(outputs[0], 5000000000, "a914c23b2cb101848e7e73459bdfcb9796c1154c25ee87");
        test_output!(outputs[1], 0, "6a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf9");

        let witnesses_n = &witnesses[0];
        test_witness!(&witnesses_n[0], "0000000000000000000000000000000000000000000000000000000000000000");

        let data = include_bytes!("tx_de06af29a80be52bb5f4b6c86998dcfdf0f9e7f66a1ebb7e9d20d65cc6785d8c.native_witness.bin");
        let (_, tx) = parse_transaction(data).unwrap();
        assert_eq!(tx.version, 1);
        assert_eq!(tx.lock_time, 0);
        assert_eq!(tx.inputs.len(), 1);
        assert_eq!(tx.outputs.len(), 2);
        let witnesses = match tx.witnesses {
        	Some(witnesses) => witnesses,
        	None => vec![]
        };
        assert_eq!(witnesses.len(), 1);
        let inputs = tx.inputs;
        test_input!(&inputs[0], "aea7e39ca42c33e20d5cb86e4a07a5607947275a1ea8bcbfde4d94bc1259d458", 4, "", 4294967295);

        let outputs = tx.outputs;
        test_output!(outputs[0], 12000000, "76a9141b6517e189434cf8f18cc38ceb88c8fdce25b8f188ac");
        test_output!(outputs[1], 6802757, "0020701a8d401c84fb13e6baf169d59684e17abd9fa216c8cc5b9fc63d622ff8c58d");

        let witnesses_n = &witnesses[0];
        test_witness!(&witnesses_n[0], "");
        test_witness!(&witnesses_n[1], "304402203b54a8f84d5e26c8dc311c6cb72de7b73a37bdd31172e1af5dde1880732a018a02202787f1d26615038ef898645f57375aa7cfb8915a43d79688b7a7647a8962f5e201");
        test_witness!(&witnesses_n[2], "30440220206ea4462d688845fd322fddadedf2ebf11fc5aedd489fcc58cd691f38c6aa3302201ff7f3c7880f4bff26bbd134e3f0adb40684b3707f8e207691fc95576edd6a8901");
        test_witness!(&witnesses_n[3], "52210375e00eb72e29da82b89367947f29ef34afb75e8654f6ea368e0acdfd92976b7c2103a1b26313f430c4b15bb1fdce663207659d8cac749a0e53d70eff01874496feff2103c96d495bfdd5ba4145e3e046fee45e84a8a48ad05bd8dbb395c011a32cf9f88053ae");

        let data = include_bytes!("tx_e73781944bc6624acf0a8ebcefa9c25046cdda8dc7ad962bb0c41bcd302f9ca5.bin");
        let (_, tx) = parse_transaction(data).unwrap();
        assert_eq!(tx.version, 1);
        assert_eq!(tx.lock_time, 0);
        assert_eq!(tx.inputs.len(), 8);
        assert_eq!(tx.outputs.len(), 5);
        let witnesses = match tx.witnesses {
        	Some(witnesses) => witnesses,
        	None => vec![]
        };
        assert_eq!(witnesses.len(), 8);
        let inputs = tx.inputs;
        test_input!(&inputs[0], "49940d3cc73e8d9b8db76729acfe4d5ebb2c575b32efa45ba6c67ab2a0221834", 3, "00483045022100c6763d39bec48c8796028c0c930de99f625ac3809d12098c7d792593e53e82020220620f164f702a29d5303419c7c0ea60993bfe76db8f1a1b22e2680053fd36db1501483045022100bb333d78951e81729bbbac62c8d9c15f1f8cddba6556a33f2976a7e0529ef21302207ccc717f8c23d152a225514f87b3b2a05de2571e852dc1633b19bb2e84bc6a5e014c69522102d0db18f7e8d8de0898cc1372ef996251daba71ad4c05eca59472a0cbd76b1fd82102535433203e2c238b29a40163193b3becfc1d57928db5b88ca04cb51033aa0e47210386eaa3ee180f8ed691777b2fa97e0d65ac43f30c4af245349f22cceb4d50fe1453ae", 4294967295);
        test_input!(&inputs[1], "bb502ca9e10f4aed9f00cd24d4f6c4eaab558c7cf2ff53e12cfcbf5cfe8b566e", 0, "004730440220654bd5bc7ecaa5c6769a7e60912a43d831aac8ab8be7acf5b64020d2dbbf2a42022001b7faf4d825cc7814df69b8d883f3643583501ae241d5cfcac8907a9706009101483045022100bcc6dfd57e3f6a49d1677567d8a232c5832f0928de1e160c557a017e731d63f702207602302e2dc164d77bffc2507aa7580c4f3650998a116fef8ab74447589dd0eb014c695221025fa13056afdc74ae19b53824ee98f6fdc9faffa2bf74850b3f3ebc4f08d3558c2102da7f163c838c4bc3b640c91738f9083317adac54becc86f4ac7c3ea7cfc002a12103d7b0d67e89fc528a4f7a5f0caef554bbb6505e2f837de23c4a7d1a42f197e92f53ae", 4294967295);
        test_input!(&inputs[2], "a32de4df3f805254ec3a81a09cc9c08835f2df5cd5907160fab198ff7260dc11", 1, "00483045022100a73fb850ab19d9a9b8ab19f402298c41280bfe2fb8982eb98789fc85bdf7071f022040725830f7e95811586fe5a754a4d1c2a04bcded1f72b4906ed4af404ea592030147304402201393951e44b02ba237d967e30b414001f55bf9c1f2b61f916415f86e0d3e1d3802201e6c82c71b42098a798ac10516756262d83d8733d3598ce106d26a43e5b10adc014c69522103c7e0c7a8761e50e32273061569aa508eaed8af32606bf5d9f48b11003db77db921037078e32eb3d25f49336ecd6dbebc69d9a2dd339c619b2210c0ca56d80a23b8482103bf55065ebbaf443480cfe96bfd63ff385ae89765d4a1b04f23b87569616be3da53ae", 4294967295);
        test_input!(&inputs[3], "521e49f8281c71ff1f1fa54403842bcb8557204fb6814f9f049f68490caaeb44", 1, "00483045022100e5f539797d8cd415181ee67cae1ec332ad21b56b92041a474a077bac3e30a714022005b5e90f0248eae9cf9978c57f345222e861052ac0a5af6318f08b155e4bea6c01473044022017eaea6cc19804906d3ed1836cb85d296161cbde19393d1b2547f9a0433698da02205aafb4b06b6df101fe6b40866eb78d43b3feed5d77496768a9f444ea0ba72408014c695221025fa13056afdc74ae19b53824ee98f6fdc9faffa2bf74850b3f3ebc4f08d3558c2102da7f163c838c4bc3b640c91738f9083317adac54becc86f4ac7c3ea7cfc002a12103d7b0d67e89fc528a4f7a5f0caef554bbb6505e2f837de23c4a7d1a42f197e92f53ae", 4294967295);
        test_input!(&inputs[4], "ebccca6415afb58c548b124471e9a4e7f79e1114b363f8adc48476a15b55753b", 0, "004730440220143061cea3b44f8c4842bfe98bea6fbb480f12706055022144d77f0e49c2a64802201e1db4fca4670bb2f509ec30b0e08a874f51e691bf87fbc91d1b7edf30b56845014730440220510d98bf8cc702803a0550e48f9f4f207878127ec06a67c3aa5a43e97ccfb59f022078d7bac28ef869168f406c4084027b858be384395ac49902679dd91190541097014c695221032e1b71c106589d21020eb03bdd07e7eb376c14354df49f507a22cf57a0dbb86b2103ab8459382dfa3172460fe5e6170fc48eabfdff9e1b147211a9fcaecd6ddd3fd42102a58f540b9d90eb643bb32c0f28c4749e3fdde2dcf1590a075e63c346c3f302b353ae", 4294967295);
        test_input!(&inputs[5], "67c4fa514a4af1a48488967903be0a9147f2456f5de20087a0f1f8da10a90328", 0, "00473044022045031c1ad4005f367481fc6145875b6911f0fea6ed0f60c3d14086b32da7520d022024de965e6ca7d33d096245dd52ce619ae275ed9760ca84380dbc68908d52595101483045022100c0ad5a8dce5a2fa3505812bb161f4dc86a47d2d7b5c73f8cfce17815666b4cb2022060a1c757b818bcef149bb940f5d1554129990966f2714ca0a446792890238ea8014c695221035099a07e0c016571164073d724e1bccf7a50882807aa49199a2f0c2e0eea487e210222db3a7c074c0750b71073e2a914659b9ea66278ba53cca2eb53e3eb6ed0d63221039b71403cff80ea043f96a9ec12f0aba0c547bc133a7a73477e0bab09bbe974d553ae", 4294967295);
        test_input!(&inputs[6], "cf395d656f71815c7f006c02a9f001712f63ac7654a80170ef842ca189431085", 7, "", 4294967295);
        test_input!(&inputs[7], "55d4194521f3d39bfa8421ff0334369f560f66604a276a363d26669d97407a8a", 9, "", 4294967295);

        let outputs = tx.outputs;
        test_output!(outputs[0], 278100000, "a914f938ef06b17f94f8943decc2de530191b8db1c3087");
        test_output!(outputs[1], 5000000000, "a914ba152510537e52bae0f0f45f0d91a81a17d4d02987");
        test_output!(outputs[2], 1000000, "76a9142fd4c4f8e61ccf9d4c9967d415bdbaeffc2a3fdc88ac");
        test_output!(outputs[3], 109500000, "a91468126269aa368cf0bfa730afcae1aec5909b0a8487");
        test_output!(outputs[4], 7588390042, "0020dca044867d191815d2e0a34b37b95f25c01d6039a58613a077a1e12e2908aab5");

        let witnesses_n = &witnesses[0];
        test_witness!(&witnesses_n[0], "");
        let witnesses_n = &witnesses[1];
        test_witness!(&witnesses_n[0], "");
        let witnesses_n = &witnesses[2];
        test_witness!(&witnesses_n[0], "");
        let witnesses_n = &witnesses[3];
        test_witness!(&witnesses_n[0], "");
        let witnesses_n = &witnesses[4];
        test_witness!(&witnesses_n[0], "");
        let witnesses_n = &witnesses[5];
        test_witness!(&witnesses_n[0], "");
        let witnesses_n = &witnesses[6];
        test_witness!(&witnesses_n[0], "");
        test_witness!(&witnesses_n[1], "3045022100c405ff924fe738675eca96421d1d63af2058051d1dee861b965ce43d903ec613022031d8d533afe0b8b6c238a37d21f5cd36c953c2cd1edcf7b8339d6c4758f9c1ae01");
        test_witness!(&witnesses_n[2], "304502210089255f13f1a4a8ab1286d773031eac50bfb49518744a2a6c8928cbbf300dbb85022055efdd2a8e89675a891ab87b9e0203c8b12301662247d65207bd82c81cc0f49e01");
        test_witness!(&witnesses_n[3], "522102e0fbda75885a685ddc1422c7bd2693db9939d0eecc1eb5a100ec089dc6cf42072103916f5f66c7c795b0256d6e7b8cfeb5803fb0fb9d4a85eb4f12be9a8b642e6cae2102067c60662a0a0ca4dc1caa390cad59162b94bcafcee27816e62b1028bbb32cd753ae");
        let witnesses_n = &witnesses[7];
        test_witness!(&witnesses_n[0], "");
        test_witness!(&witnesses_n[1], "304402203c041cd88eb282d51f510e28cc740c4673a2618175c1dad1d598017f9e2cad080220128cf72146cd5adab73a14002bbb07d28d4a0e2ac33ef878758764f8ca5ac29f01");
        test_witness!(&witnesses_n[2], "30450221008dc95a50bb012e475ed45fd63069217c33406380435ef3f395a1ef6cf1f521730220008f65db6be357d6fa1393dab1f2d496eace06cf94091cd8a13906f2a3bc001901");
        test_witness!(&witnesses_n[3], "522102c030f9a440306237ad33114a8aef0467580feb630879c14fbfe4cb32e021f1de2102779c6ec58e8424d2d7877351e6d7c5011cf420e1818917337ee5e7b3d0f564de2103c5ce7515acc2f25159d6c2e19d76c4ee02eb625e517e59216729ce34bdb72a3253ae");

        let data = include_bytes!("tx_fb042de1f26d3ea4df6a5d7c7b8bb3463d49ac32400df4b881ad87d922a6be54.segwit.bin");
        let (_, tx) = parse_transaction(data).unwrap();
        assert_eq!(tx.version, 2);
        assert_eq!(tx.lock_time, 0);
        assert_eq!(tx.inputs.len(), 3);
        assert_eq!(tx.outputs.len(), 2);
        let witnesses = match tx.witnesses {
        	Some(witnesses) => witnesses,
        	None => vec![]
        };
        assert_eq!(witnesses.len(), 3);
        let inputs = tx.inputs;
        test_input!(&inputs[0], "9715af8a302c0cd4e61bfc36dd07e121587ae0a658aa282221620d0731759308", 7, "1600144d3e60e105bfa848ecee7a5c3ce3813daea667d1", 4294967295);
        test_input!(&inputs[1], "9ce8f610df9690fb2984db5899a9b2682f65ece021e6a31d826c9c8a0f48946e", 0, "160014e1cd4fddae7903830809211dafc8b7d8ab5afa32", 4294967295);
        test_input!(&inputs[2], "a0f144c434bf1ad4d9e98e0bc0209f2ac28037788741d15db8f12d0c085fe362", 1, "160014f7bae6ee31d59e79da8e857fb63429637f9e0d57", 4294967295);

        let outputs = tx.outputs;
        test_output!(outputs[0], 2500000, "76a9147e7622d8d0efdb8d70ce09778dbbbf458459dec388ac");
        test_output!(outputs[1], 203232, "a914f03e6bf9b389bbd5d5669ff55c4dba30de99553587");

        let witnesses_n = &witnesses[0];
        test_witness!(&witnesses_n[0], "30440220031da3a5c42846d0f06b60a7d2ff36b660c1796e6f39819ddab1f40d6ee695ca02204af5aaf048b97b9148ee819781bf562898097001994fbb8c30a07be54e953d5c01");
        test_witness!(&witnesses_n[1], "036ae671cef76a3d484b870750035732dbe1b375e63025bc4d41c497c88dfe0250");
        let witnesses_n = &witnesses[1];
        test_witness!(&witnesses_n[0], "304402206f544285f5ac334fc9ced76a018263af0cdc4a1b6dcfa65c53345ad3d177399e02206ff079d82f89ff524ab3cf62f4b9190655106db7109efe12dea75556468c00bb01");
        test_witness!(&witnesses_n[1], "02b4ef840cdc831ba64efa3148658c9846edc9a6d46135afad22e8cfe0ebdda13c");
        let witnesses_n = &witnesses[2];
        test_witness!(&witnesses_n[0], "30440220128f1b958bdd4696d0f9d11f6909bceaac5c9ae36a0b61f1a12cf21853ba696402202027c26b3bf790674f3b910e41963f54e36b82e11092016b2f01e09310e835da01");
        test_witness!(&witnesses_n[1], "034ed258709969db6507e5b86568e6c57d903917034a853fd2f6b7604443431518");

    }
    #[test]
    fn test_parse_block (){
        let data = include_bytes!("blk.0000000000000000000b0a682f47f187a712c42badd4ca1989c494d401457c3f.bin");
        let (_, block) = parse_block(data).unwrap();
        assert_eq!(block.transactions.len(),2996);
        // let res = parse_block_header(data);
        // #[derive(Debug)]
        // enum Test<T, K,L> {
        //     Result(T),
        //     Error(K),
        //     Incomplete(L)
        // }
        // let ize = match res {
        //     Ok((_,o)) => format!("Output: {:?}",o),
        //     Err(e) => {
        //         match e {
        //             nom::Err::Error((_,e)) => format!("Error {:?}",e),
        //             nom::Err::Failure(e) => format!("Failuer {:?}", e),
        //             nom::Err::Incomplete(n) => format!("Needed {:?}",e)
        //         }
        //     }
        // };
        // println!("ize: {}",ize);
        // assert_eq!(1,0);

        // println!("{:?}", block.header);
        // assert_eq!(block.size, 1);
    }
}
