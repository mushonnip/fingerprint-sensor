use fingerprint_sensor::{
    Device, BADLOCATION, ENROLLMISMATCH, FEATUREFAIL, FLASHERR, IMAGEFAIL, IMAGEMESS, INVALIDIMAGE,
    NOFINGER, OK,
};
use serialport::{self};
use std::io::{self, Write};
use std::process::exit;
use std::thread::sleep;
use std::time::Duration;

fn get_fingerprint(device: &mut Device) -> io::Result<()> {
    if device.get_image()? != OK {
        return Err(io::Error::new(io::ErrorKind::Other, "Failed to get image"));
    }

    println!("Templating...");

    if device.image_2_tz(1)? != OK {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to convert image to template",
        ));
    }

    println!("Searching...");

    if device.finger_search()? != OK {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to search for fingerprint",
        ));
    } else {
        return Ok(());
    }
}

fn get_num(max_number: u16) -> u16 {
    loop {
        print!("Enter ID # from 0-{}: ", max_number - 1);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        match input.trim().parse::<u16>() {
            Ok(i) if i < max_number => return i,
            _ => println!(
                "Invalid input. Please enter a number between 0 and {}.",
                max_number - 1
            ),
        }
    }
}

fn enroll_finger(location: u16, device: &mut Device) -> io::Result<()> {
    for fingerimg in 1..=2 {
        if fingerimg == 1 {
            print!("Place finger on sensor...");
        } else {
            print!("Place same finger again...");
        }

        loop {
            let i = device.get_image()?;
            match i {
                OK => {
                    println!("Image taken");
                    break;
                }
                NOFINGER => print!("."),
                IMAGEFAIL => {
                    println!("Imaging error");
                    return Err(io::Error::new(io::ErrorKind::Other, "Imaging error"));
                }
                _ => {
                    println!("Other error");
                    return Err(io::Error::new(io::ErrorKind::Other, "Other error"));
                }
            }
        }

        print!("Templating...");
        let i = device.image_2_tz(fingerimg)?;
        match i {
            OK => println!("Templated"),
            IMAGEMESS => {
                println!("Image too messy");
                return Err(io::Error::new(io::ErrorKind::Other, "Image too messy"));
            }
            FEATUREFAIL => {
                println!("Could not identify features");
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Could not identify features",
                ));
            }
            INVALIDIMAGE => {
                println!("Image invalid");
                return Err(io::Error::new(io::ErrorKind::Other, "Image invalid"));
            }
            _ => {
                println!("Other error");
                return Err(io::Error::new(io::ErrorKind::Other, "Other error"));
            }
        }

        if fingerimg == 1 {
            println!("Remove finger");
            sleep(Duration::from_secs(1));
            let img = match device.get_image() {
                Ok(i) => i,
                Err(_) => return Err(io::Error::new(io::ErrorKind::Other, "Failed to get image")),
            };

            while img != NOFINGER {}
        }
    }

    print!("Creating model...");
    let i = device.create_model()?;
    match i {
        OK => println!("Created"),
        ENROLLMISMATCH => {
            println!("Prints did not match");
            return Err(io::Error::new(io::ErrorKind::Other, "Prints did not match"));
        }
        _ => {
            println!("Other error");
            return Err(io::Error::new(io::ErrorKind::Other, "Other error"));
        }
    }

    print!("Storing model #{}...", location);
    let i = device.store_model(location as u16, 1)?;
    match i {
        OK => println!("Stored"),
        BADLOCATION => {
            println!("Bad storage location");
            return Err(io::Error::new(io::ErrorKind::Other, "Bad storage location"));
        }
        FLASHERR => {
            println!("Flash storage error");
            return Err(io::Error::new(io::ErrorKind::Other, "Flash storage error"));
        }
        _ => {
            println!("Other error");
            return Err(io::Error::new(io::ErrorKind::Other, "Other error"));
        }
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let port_name = "/dev/ttyS3";
    let baud_rate = 57600;

    let uart = serialport::new(port_name, baud_rate)
        .timeout(Duration::from_millis(900))
        .open()
        .expect("Failed to open serial port");

    let address = vec![0xFF; 4];
    let password = vec![0; 4];
    let mut device = Device::new(address, password, uart);

    match device.count_templates() {
        Ok(_) => println!("Template count: {}", device.template_count),
        Err(e) => println!("Failed to count templates: {}", e),
    }

    loop {
        println!("e) enroll print");
        println!("f) find print");
        println!("d) delete print");
        println!("q) quit");
        println!("----------------");
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        let c = input.trim();

        match c {
            "e" => {
                let library_size = match device.library_size {
                    Some(size) => size,
                    None => {
                        println!("Library size not set");
                        continue;
                    }
                };
                println!("{}", library_size);
                match enroll_finger(get_num(library_size), &mut device) {
                    Ok(_) => println!("Enrolled"),
                    Err(_) => println!("Failed to enroll"),
                }
            }
            "f" => {
                println!("Waiting for image...");
                loop {
                    match get_fingerprint(&mut device) {
                        Ok(_) => {
                            println!(
                                "Detected #{} with confidence {}",
                                device.finger_id, device.confidence
                            );
                            break;
                        }
                        Err(_) => (),
                    }
                }
            }
            "d" => {
                let finger_id = match device.library_size {
                    Some(size) => get_num(size),
                    None => {
                        println!("Library size not set");
                        continue;
                    }
                };

                match device.delete_model(finger_id) {
                    Ok(_) => println!("Deleted!"),
                    Err(_) => println!("Failed to delete"),
                }
            }
            "q" => {
                println!("Exiting fingerprint example program");
                exit(0);
            }
            _ => println!("Invalid option, please try again"),
        }
    }
}
