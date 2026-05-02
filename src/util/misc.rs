use crate::args::ARGS;
use linkify::{LinkFinder, LinkKind};
use qrcode_generator::QrCodeEcc;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::pasta::Pasta;
use std::collections::HashMap;

pub fn remove_expired(pastas: \u0026mut HashMap\u003cString, Pasta\u003e) {
    let timenow: i64 = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) =\u003e n.as_secs(),
        Err(_) =\u003e {
            log::error!(\"SystemTime before UNIX EPOCH!\");
            0
        }
    } as i64;

    let mut to_remove = Vec::new();

    for (slug, p) in pastas.iter() {
        // Expired or burn limit reached
        let expired = p.expiration \u003e 0 \u0026\u0026 p.expiration \u003c timenow;
        let burned = p.burn_after_reads \u003e 0 \u0026\u0026 p.read_count \u003e= p.burn_after_reads;
        
        // GC check
        let days_since_read = (timenow - p.last_read) / 86400;
        let inactive = ARGS.gc_days \u003e 0 \u0026\u0026 days_since_read \u003e= ARGS.gc_days as i64;

        if expired || burned || inactive {
            to_remove.push(slug.clone());
        }
    }

    for slug in to_remove {
        pastas.remove(\u0026slug);
    }
}

pub fn string_to_qr_svg(str: \u0026str) -\u003e String {
    qrcode_generator::to_svg_to_string(str, QrCodeEcc::Low, 256, None::\u003c\u0026str\u003e).unwrap()
}

pub fn is_valid_url(url: \u0026str) -\u003e bool {
    let finder = LinkFinder::new();
    let spans: Vec\u003c_\u003e = finder.spans(url).collect();
    !spans.is_empty() \u0026\u0026 spans[0].as_str() == url \u0026\u0026 Some(\u0026LinkKind::Url) == spans[0].kind()
}
