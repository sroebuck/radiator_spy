use bitvec::order::Msb0;
use bitvec::prelude::*;
use bitvec::vec::BitVec;
use cc1101::{AddressFilter, Cc1101, Modulation, PacketLength, RadioMode, SyncMode};
use colorful::Color;
use colorful::Colorful;
use itertools::Itertools;
use rppal::gpio::{Gpio, OutputPin};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{thread, time};

mod iterreader;

type RadioErr = cc1101::Error<rppal::spi::Error, ()>;

fn configure_radio(spi: Spi, cs: OutputPin) -> Result<Cc1101<Spi, OutputPin>, RadioErr> {
    let mut cc1101 = Cc1101::new(spi, cs)?;

    cc1101.set_defaults()?;
    cc1101.additional_settings()?;
    cc1101.set_frequency(868_350_000u64)?;
    // On/Off Keying OOK - not ASK
    cc1101.set_modulation(Modulation::OnOffKeying)?;
    // I've disabled this but we may need to use this to sync with the start of a signal.
    // We would be looking for 12 zeros followed by a 1, where each zero would really be
    // "1100", so that would be 48 readings.
    // cc1101.set_sync_mode(SyncMode::Disabled)?;
    cc1101.set_sync_mode(SyncMode::MatchPartial(0b1100110011001100))?;
    // Currently set to 30 to make sure it's big enough, not sure what we actually need.
    cc1101.set_packet_length(PacketLength::Fixed(40))?;
    // I don't think this applies to what we are doing
    cc1101.set_address_filter(AddressFilter::Disabled)?;
    // I think this represents the extent to which the signal might deviate from the intended
    // frequency.
    cc1101.set_deviation(2_000)?;
    // Baud rate of channel... maybe this is for the SPI connection.
    // If it's for the data then it might make sense for it to be a multiple of frequency of
    // changes in the signal which is once every 200us.  So that would be a baud rate with a
    // multiple of 5000.  The CC1101 can go up to 250,000 baud for OOK/ASK.
    // cc1101.set_data_rate(4_990)?;
    cc1101.set_data_rate(5_000)?;
    // Channel Bandwidth:
    // cc1101.set_chanbw(500)?;
    // TODO: Fix the underlying convert channel spacing function before reactivating this.
    // cc1101.set_channel_spacing(199584)?;

    let _ = cc1101.write_registers(&mut std::io::stdout());
    println!();
    let _ = cc1101.write_settings(&mut std::io::stdout());
    println!();

    Ok(cc1101)
}

fn receive_packet(cc1101: &mut Cc1101<Spi, OutputPin>) -> Result<(), RadioErr> {
    // TODO: Remove the specific processing for the old use of this code and just read the raw
    // data and display until we get a sense of whether thigns are working.

    cc1101.set_radio_mode(RadioMode::Receive)?;

    thread::sleep(time::Duration::from_millis(10));

    let mut dst = 0u8;
    let mut payload = [0u8; 300];

    let (length, _lqi) = cc1101.receive_without_crc(&mut dst, &mut payload)?;
    let rssi = cc1101.get_rssi_dbm()?;
    let lqi2 = cc1101.get_lqi()?;
    // only display high power signals...
    println!("rssi: {}, lqi: {}", rssi, lqi2);

    println!(
        "payload[{}]: {:?}",
        length,
        payload
            .iter()
            .take(length as usize)
            .map(|b| format!("{:08b}", b))
            .collect::<String>()
    );
    let bv = BitVec::<Msb0, u8>::from_slice(&payload[..length as usize]);
    let results = on_offs_to_bits(bv);
    for i in 0..results.len() {
        let result = results[i].clone();
        if result.len() > 5 * 8 {
            println!(
                "Before filtering: {}",
                format!("{:?}", result).color(Color::Green)
            );
        }
        let synced_bits = sync_bits(result);
        println!("Synced: {:?}", synced_bits);
        let bytes = chunk_into_bytes(synced_bits);
        if bytes.len() >= 6 {
            println!("Chunked: {}", format!("{:?}", bytes).color(Color::Magenta));
        } else {
            println!("Chunked: {:?}", bytes);
        }
        if let Some(signal) = decode_bytes_as_signal(bytes) {
            println!("\n{}: {}", i, format!("{:?}", signal).color(Color::Red));
        }
    }

    Ok(())
}

#[test]
fn test_bitvec() {
    let x = bitvec![Msb0,u8; 0, 0, 1, 0, 0, 1, 1, 1];
    let y: u8 = x.load_be();
    assert_eq!(39, y);
}

#[test]
fn test_on_offs_to_bits() {
    let bits = "1100110011001100110011001100110011001100110011001110001100110011100011001\
    1001110001110001110001100110011001100110011100011001100111000110011001100110011001100110011\
    0011001100110011001110001100111000111000110011001110001110001110001110001100111000110011100\
    011100011001100111000110011100011001100111000111000110011";
    let bitvec: BitVec<Msb0, u8> = bits.chars().map(|c| c == '1').collect();
    let result = on_offs_to_bits(bitvec);
    pretty_assertions::assert_eq!(
        result[0].to_string(),
        "[00000000, 00001001, 00111000, 00100100, 00000000, 00101100, 11110101, 10010100, 110]"
    );
}

#[test]
fn text_on_offs_to_multibits() {
    let bits = "111110111011101110111111110111111110111101111101111111111111011011111111111011101101110111111110010100111111011011011100011010100001110011101001011110110111011101011110110111111011010001110011010011000100111001001110001110111100101110100000000011001110110010011101111001110001001110010100110010010000101111011000101110001001010111011100011000111100011001101000000010000010101011101010111001001110111110110010111111111001000001110100000000011110011101111111110101100110110100111110010000010000011001001001110001101111101111111111101111111101111111101111011111011111111111110110111111111110111011011101111111100101001111110110110111000110101000011100111010010111101101110111010111101101111110110100011100110100110001001110010011100011101111001011101000000000110011101100100111011110011100010011100101001100100100001011110110001011100010010101110111000110001111000110011010000000100000101010111010101110010011101111101100101111111110010000011101000000000111100111011111111101011001101101001111100100000100000110010010011100011011111011111110111111111110111111110111111110111101111101111111111111011011111111111011101101110111111110010100111111011011011100011010100001110011101001011110110111011101011110110111111011010001110011010011000100111001001110001110111100101110100000000011001110110010011101111001110001001110010100110010010000101111011000101110001001010111011100011000111100011001101000000010000010101011101010111001001110111110110010111111111001000001110100000000011110011101111111110101100110110100111110010000010000011001001001110001101111101111111111101111111101111111101111011111011111111111110110111111111110111011011101111111100101001111110110110111000110101000011100111010010111101101110111010111101101111110110100011100110100110001001110010011100011101111001011101000000000110011101100100111011110011100010011100101001100100100001011110110001011100010010101110111000110001111000110011010000000100000101010111010101110010011101111101100101111111110010000011101000000000111100111011111111101011001101101001111100100000100000110";
    let bitvec: BitVec<Msb0, u8> = bits.chars().map(|c| c == '1').collect();
    let result = on_offs_to_bits(bitvec);
    assert2::check!(result.len() == 1);
    pretty_assertions::assert_eq!(
        result[0].to_string(),
        "[00000000, 00001001, 00111000, 00100100, 00000000, 00101100, 11101111, 11010001, 110]"
    );
    // pretty_assertions::assert_eq!(
    //     result[1].to_string(),
    //     "[00000000, 00100000, 00001001, 00111010, 11001001, 11101111, 0]"
    // );
    // pretty_assertions::assert_eq!(
    //     result[2].to_string(),
    //     "[00000000, 00100000, 00000100, 10011101, 01100100, 11110111, 10]"
    // );
}

#[test]
fn test_sync_bits1() {
    let bits = "0000000000001001001110000010010000000000001011001111010110010100110";
    let bitvec: BitVec<Msb0, u8> = bits.chars().map(|c| c == '1').collect();
    let result = sync_bits(bitvec);
    assert_eq!(
        "[00100111, 00000100, 10000000, 00000101, 10011110, 10110010, 100110]",
        result.to_string()
    );
}

#[test]
fn test_sync_bits2() {
    let bits = "0000010010011100000100100000000000010110011100111100000101100";
    let bitvec: BitVec<Msb0, u8> = bits.chars().map(|c| c == '1').collect();
    let result = sync_bits(bitvec);
    assert_eq!(
        "[00100111, 00000100, 10000000, 00000101, 10011100, 11110000, 0101100]",
        result.to_string()
    );
}

#[test]
fn test_chunk_into_bytes() {
    // 001001110 000010010 000000000 001011001 111010110 010100110
    // let bits = "001001110000010010000000000001011001111010110010100110";
    let bits = "00001001110000010010000000000001111000000011011100001011";
    let bitvec: BitVec<Msb0, u8> = bits.chars().map(|c| c == '1').collect();
    let result = chunk_into_bytes(bitvec);
    assert_eq!("[39, 9, 0, 60, 13, 133]", format!("{:?}", result));
}

#[derive(Debug, PartialEq)]
enum FHTCommand {
    Sync,
    OpenTo(u8),
}

#[derive(Debug, PartialEq)]
struct FHTSignal {
    house_code1: u8,
    house_code2: u8,
    address: u8,
    command: FHTCommand,
}

#[test]
fn test_decode_bytes() {
    // let bytes: Vec<u8> = vec![39, 9, 0, 44, 235, 83];
    let bytes = vec![39, 5, 0, 182, 227, 209];
    let signal = decode_bytes_as_signal(bytes).unwrap();
    assert_eq!(
        FHTSignal {
            house_code1: 39,
            house_code2: 5,
            address: 0,
            command: FHTCommand::OpenTo(89),
        },
        signal
    );
}

fn decode_bytes_as_signal(bytes: Vec<u8>) -> Option<FHTSignal> {
    if bytes.len() <= 5 {
        return None;
    }
    let command_byte = bytes[3];
    let has_extension = command_byte >> 5 & 1 == 1;
    let len = match has_extension {
        false => 5,
        true => 6,
    };
    if bytes[len - 1]
        != ((bytes[0..len - 1].iter().map(|&b| b as u64).sum::<u64>() + 0x0C) & 0xFF) as u8
    {
        return None;
    }

    let command = match command_byte & 0xF {
        0x6 => FHTCommand::OpenTo((bytes[4] as u32 * 100 / 255) as u8),
        0xC => FHTCommand::Sync,
        _ => {
            println!("UNKNOWN COMMAND!");
            return None;
        }
    };

    let signal = FHTSignal {
        house_code1: bytes[0],
        house_code2: bytes[1],
        address: bytes[2],
        command,
    };
    Some(signal)
}

/// Take a sequence of on and off bits and convert them using the rull that 111000 represents 1 and
/// 1100 represents 0.  Because there is noise in the system the challenge is in recovering the
/// sequence of bits correctly after some error values and splitting the sequence at every point
/// where it clearly goes wrong in order to make sure that the output is always a correct
/// reflection of a continuous sequence of bits.
fn on_offs_to_bits(on_offs: BitVec<Msb0, u8>) -> Vec<BitVec<Msb0, u8>> {
    // let mut result = BitVec::<Msb0, u8>::new();
    let mut result: Vec<u8> = Vec::new();
    on_offs
        .iter()
        .fold((0u16, 0u16), |true_false, bit| match bit {
            true => match true_false {
                (4, _) => {
                    result.push(2);
                    (4, 0)
                }
                (t, _) => (t + 1, 0),
            },
            false => match true_false {
                (0, _) => (0, 1),
                (2, 1) | (4, 1) => {
                    result.push(0);
                    (0, 0)
                }
                (_, 2) => {
                    result.push(1);
                    (0, 0)
                }
                (t, f) => (t, f + 1),
            },
        });
    let split_iters = result.split(|&n| n == 2).filter(|e| e.len() > 8 * 3);
    let x: Vec<Vec<u8>> = split_iters.map(|s| s.to_vec()).collect();
    x.iter()
        .map(|v| v.iter().map(|&i| i == 1).collect::<BitVec<Msb0, u8>>())
        .collect()
}

fn sync_bits(bits: BitVec<Msb0, u8>) -> BitVec<Msb0, u8> {
    let mut result = BitVec::<Msb0, u8>::new();
    bits.iter()
        .fold((0u16, false), |(zeros, is_go), &bit| match is_go {
            true => {
                result.push(bit);
                (0, true)
            }
            false => match (zeros, bit) {
                (z, false) => (z + 1, false),
                (z, true) if z >= 2 => (0, true),
                (_, true) => (0, false),
            },
        });
    result
}

fn chunk_into_bytes(bits: BitVec<Msb0, u8>) -> Vec<u8> {
    let chunks = bits.iter().chunks(9);
    // Treat as 9bit chunks and reject values with incorrect final check digit
    let bytes: Vec<_> = chunks
        .into_iter()
        .map(|c| c.map(|&c| c).collect::<BitVec<Msb0, u8>>())
        .take_while(|c| c.iter().filter(|&&b| b).count() % 2 == 0)
        .map(|mut c| {
            c.truncate(8);
            c
        })
        .map(|c| c.load_be::<u8>())
        .collect();
    bytes
}

fn main() -> Result<(), RadioErr> {
    let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 50_000, Mode::Mode0).unwrap();
    let cs = Gpio::new().unwrap().get(8).unwrap().into_output();

    let mut cc1101 = configure_radio(spi, cs)?;

    println!("Len ID Cnt Status Fixed    PCnt AvgTime PulseCnt ?? RSSI LQI");
    println!("--- -- --- ------ -----    ---- ------- -------- -- ---- ---");

    loop {
        if let Err(err) = receive_packet(&mut cc1101) {
            println!("Error: {:?}", err);
        }
    }
}
