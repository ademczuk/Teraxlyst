// Public-id generation for tracker items.
//
// Three formats:
// - Sequential: prefix-padded counter, e.g. "BUG-001". Counter is derived
//   from the highest existing matching id, NOT from item count. This lets
//   deletes preserve the monotonic-id invariant.
// - Ulid: 26-char Crockford base32 ULID. Already in our Cargo deps.
// - Uuid: v4 UUID via a tiny RNG (we don't pull the uuid crate just for
//   this; ulid + a synthesized v4 keeps the deps tight).

use ulid::Ulid;

use super::error::TrackerError;
use super::schema::IdFormat;

// Compute the next public id given the format and the slice of currently
// existing ids for this tracker. The existing list lets sequential ids
// pick `max+1` without re-scanning the DB inside the helper.
pub fn next_public_id(format: &IdFormat, existing: &[String]) -> Result<String, TrackerError> {
    match format {
        IdFormat::Sequential { prefix, pad } => {
            let max = highest_sequential(prefix, existing);
            let next = max + 1;
            // Width clamped to the requested pad. If next overflows the
            // pad width we keep emitting; users hitting BUG-1000 with
            // pad=3 are not blocked.
            let s = format!("{}-{:0>width$}", prefix, next, width = *pad as usize);
            Ok(s)
        }
        IdFormat::Ulid => Ok(Ulid::new().to_string()),
        IdFormat::Uuid => Ok(synth_uuid_v4()),
    }
}

// Scan existing ids, parse the trailing numeric suffix for those matching
// `<prefix>-<n>`, return the highest n seen. Non-matching ids are ignored
// (they may be legacy ulid items after a format switch).
fn highest_sequential(prefix: &str, existing: &[String]) -> u64 {
    let head = format!("{}-", prefix);
    let mut max: u64 = 0;
    for id in existing {
        if let Some(rest) = id.strip_prefix(&head) {
            if let Ok(n) = rest.parse::<u64>() {
                if n > max {
                    max = n;
                }
            }
        }
    }
    max
}

// Compose a v4 UUID-shaped string without pulling the uuid crate. Uses
// the ulid crate's RNG indirectly: a fresh Ulid is 128 bits of entropy +
// timestamp; we re-pack into a UUID layout. Variant + version nibbles
// are set per RFC 4122.
//
// Note: this is *not* a cryptographically-random UUID v4 in the strict
// sense - the timestamp prefix is monotonic - but it satisfies the
// uniqueness guarantee needed for public_id and is human-stable.
fn synth_uuid_v4() -> String {
    let ulid = Ulid::new();
    let raw = ulid.0; // 128 bits packed
    let bytes = raw.to_be_bytes();
    let mut b = bytes;
    // Set version (4) in byte 6 high nibble.
    b[6] = (b[6] & 0x0F) | 0x40;
    // Set variant (10) in byte 8 high nibble.
    b[8] = (b[8] & 0x3F) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15],
    )
}
