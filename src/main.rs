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
    // cc1101.set_sync_mode(SyncMode::MatchFull(0xCCCC))?;
    cc1101.set_sync_mode(SyncMode::Disabled)?;
    // Currently set to 30 to make sure it's big enough, not sure what we actually need.
    cc1101.set_packet_length(PacketLength::Variable(32))?;
    // I don't think this applies to what we are doing
    cc1101.set_address_filter(AddressFilter::Disabled)?;
    // I think this represents the extent to which the signal might deviate from the intended
    // frequency.
    cc1101.set_deviation(50_000)?;
    // Baud rate of channel... maybe this is for the SPI connection.
    // If it's for the data then it might make sense for it to be a multiple of frequency of
    // changes in the signal which is once every 200us.  So that would be a baud rate with a
    // multiple of 5000.  The CC1101 can go up to 250,000 baud for OOK/ASK.
    cc1101.set_data_rate(4_990)?;
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
    if rssi > -80 {
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
        // Decode 111000 => 1 and 1100 => 0
        let mut result = BitVec::<Msb0, u8>::new();
        let _final_state = bv.iter().fold((0u16, 0u16), |s, b| match b {
            true => match s {
                (3, _) => (4, 0),
                (t, _) => (t + 1, 0),
            },
            false => match s {
                (0, f) => (0, f + 1),
                (2, 1) => {
                    result.push(false);
                    (0, 0)
                }
                (3, 2) => {
                    result.push(true);
                    (0, 0)
                }
                (_, 2) => {
                    result.push(true);
                    (0, 0)
                }
                (t, f) => (t, f + 1),
            },
        });
        println!("Before filtering: {:?}", result);
        // Treat as 9bit chunks and reject values with incorrect final check digit
        let chunks = result.iter().skip_while(|&&b| b == false).skip(1).chunks(9);
        let bytes: Vec<_> = chunks
            .into_iter()
            // .map(|c| format!("{:?}", c.map(|&b| b as u8).collect::<Vec<u8>>()))
            .map(|c| c.map(|&c| c).collect::<BitVec<Msb0, u8>>())
            .take_while(|c| c.iter().filter(|&&b| b).count() % 2 == 0)
            .map(|mut c| {
                c.truncate(8);
                c
            })
            .map(|c| c.load_be::<u8>())
            .collect();
        if bytes.len() > 0 && bytes[0] == 39 {
            println!("{}", format!("Filtered: {:?}", bytes).color(Color::Green));
        } else {
            println!("{}", format!("Filtered: {:?}", bytes).color(Color::Red));
        }
    // .into_iter()
    // .map(|c| c.bits())
    // .take_while(|c| c.into_iter().map(|&b| b as u8).sum::<u8>() % 2 == 0)
    // .map(|c| format!("{:?}", c))
    // .collect();
    } else {
        println!(".");
    }

    Ok(())
}

#[test]
fn test_bitvec() {
    let x = bitvec![Msb0,u8; 0, 0, 1, 0, 0, 1, 1, 1];
    let y: u8 = x.load_be();
    assert_eq!(39, y);
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
