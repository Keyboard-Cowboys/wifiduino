use std::env;
use std::fs::File;
use std::io::Write;
use std::time::Duration;

use base64::encode;
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, SET_COOKIE};
use reqwest::Error;

use scraper::{Html, Selector};

use qrcode::render::svg;
use qrcode::QrCode;

const DARK: svg::Color = svg::Color("#246491");
const LIGHT: svg::Color = svg::Color("#f9f1df");

const IMG_SIZE: u32 = 600;

fn generate_qr_code(ssid: &str, password: &str) -> QrCode {
    let data = format!("WIFI:S:{};T:WPA2;P:{};;", ssid, password);

    return QrCode::new(&data).unwrap();
}

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();

    let client: Client = Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let username: String = "admin".to_string();
    let password: String = args[1].to_owned(); // FIX: unsafe arg read. Running w/o passing the password as an arg will fail

    let basic_auth_val: String = encode(format!("{}:{}", username, password));

    let mut headers: HeaderMap = HeaderMap::new();

    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Basic {}", basic_auth_val)).unwrap(),
    );

    let req: RequestBuilder = client
        .get("https://192.168.1.1/WLG_wireless_dual_band_2.htm") // TODO: abstract this, or make it a runtime argument
        .headers(headers.to_owned());

    // Perform initial auth'd request. Cookies will be set to client here
    match req.try_clone().unwrap().send() {
        Ok(res) => match res.headers().get(SET_COOKIE) {
            Some(token) => println!("Fetched XSRF TOKEN: {:?}", token),
            _ => eprintln!("XSRF TOKEN not found! {:?}", res.headers()),
        },
        Err(e) => eprintln!("Failed to fetch xsrf token: {}", e),
    }

    match req.try_clone().unwrap().send() {
        Ok(res) => {
            match res.text() {
                Ok(html) => {
                    let document: Html = Html::parse_document(&html);

                    let ssid_selector: Selector = Selector::parse("#ssid").unwrap();
                    let pass_selector: Selector = Selector::parse("#passphrase").unwrap();

                    let ssid: String = document
                        .to_owned()
                        .select(&ssid_selector)
                        .next()
                        .unwrap()
                        .value()
                        .attr("value")
                        .unwrap()
                        .to_string();
                    let password: String = document
                        .to_owned()
                        .select(&pass_selector)
                        .next()
                        .unwrap()
                        .value()
                        .attr("value")
                        .unwrap()
                        .to_string();

                    let qrcode: QrCode = generate_qr_code(&ssid, &password);

                    // Render the bits into an image.
                    let image = qrcode
                        .render()
                        .min_dimensions(IMG_SIZE, IMG_SIZE)
                        .dark_color(DARK)
                        .light_color(LIGHT)
                        .build();
                    
                    // Writing an SVG file. Resize it however you like from there.
                    let mut file: File = File::create("qr.svg").unwrap();

                    let write_status = file.write_all(image.as_bytes());

                    match write_status {
                        Ok(_) => println!("Write Complete"),
                        Err(e) => eprintln!("Error writing file: {}", e),
                    }
                }
                Err(e) => eprintln!("Error parsing response: {}", e),
            }
        }
        Err(e) => eprintln!("Error reaching router: {}", e),
    }

    Ok(())
}
