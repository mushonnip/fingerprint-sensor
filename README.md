# fingerprint-sensor

[![Docs](https://docs.rs/fingerprint-sensor/badge.svg)](https://docs.rs/fingerprint-sensor)
[![crates.io](https://img.shields.io/crates/v/fingerprint-sensor.svg)](https://crates.io/crates/fingerprint-sensor)

fingerprint-sensor is a Rust library for interfacing with fingerprint sensors. This project aims to provide an easy-to-use API for biometric authentication and fingerprint sensor interaction, especially in embedded environments.

## Features
- Enroll and store fingerprints in the sensor's memory.
- Match fingerprints against stored templates.
- Delete specific fingerprints or clear all stored templates.

## Installation
```bash
cargo add fingerprint-sensor
```

## Usage
See the `examples` folder for a simple example of how to use the library.

## Supported Sensors
This library is designed to work with UART-based fingerprint sensors. Compatibility with specific models will be documented as development progresses. In this time, I can confirm that the following sensors are supported:
- AS608 Optical Finger Print Sensor Module

## License
This project is released under the [MIT License](LICENSE).

## Acknowledgments
- [Adafruit CircuitPython Fingerprint](https://github.com/adafruit/Adafruit_CircuitPython_Fingerprint) for inspiration and reference implementation.