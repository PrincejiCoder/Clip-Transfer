use crate::args::ARGS;
use linkify::{LinkFinder, LinkKind};
use qrcode_generator::QrCodeEcc;
use chrono::Utc;
use crate::pasta::Pasta;
use std::collections::HashMap;

pub fn remove_expired(pastas: &mut HashMap<String, Pasta>) {
    let now = Utc::now();

    pastas.retain(|_, p| {
        // Expiration is inclusive: remove if expiry <= now
        let expired = p.expiration.map_or(false, |e| e <= now);
        
        // Burn limit reached
        let burned = p.burn_after_reads > 0 && p.read_count >= p.burn_after_reads;
        
        // GC check (inactivity)
        let days_since_read = (now - p.last_read).num_days();
        let inactive = ARGS.gc_days > 0 && days_since_read >= ARGS.gc_days as i64;

        !expired && !burned && !inactive
    });
}

pub fn string_to_qr_svg(str: &str) -> String {
    // Limit input length to maintain scanability (Data Density Guard)
    let input = if str.len() > 1024 {
        log::warn!("QR input exceeds 1024 characters, truncating for scanability");
        &str[..1024]
    } else {
        str
    };

    match qrcode_generator::to_svg_to_string(input, QrCodeEcc::Low, 256, None::<&str>) {
        Ok(svg) => {
            // Post-process to ensure preserveAspectRatio and responsive attributes
            let svg = svg.replace("<svg ", "<svg preserveAspectRatio=\"xMidYMid meet\" ");
            // Ensure viewBox exists (qrcode-generator usually adds it, but let's be safe)
            if !svg.contains("viewBox") {
                svg.replace("<svg ", "<svg viewBox=\"0 0 256 256\" ")
            } else {
                svg
            }
        },
        Err(e) => {
            log::error!("QR Generation failed: {:?}", e);
            format!("<div class='qr-fallback'>CODE: {}</div>", str)
        }
    }
}

pub fn is_valid_url(url: &str) -> bool {
    let finder = LinkFinder::new();
    let spans: Vec<_> = finder.spans(url).collect();
    !spans.is_empty() && spans[0].as_str() == url && Some(&LinkKind::Url) == spans[0].kind()
}
