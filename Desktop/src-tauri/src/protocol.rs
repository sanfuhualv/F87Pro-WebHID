use crate::model::{Frame, Key};
pub fn validate_report(raw: &[u8]) -> Result<[u8; 20], String> {
    if raw.len() != 20
        || raw[0] != 0x13
        || ![0x88, 0x44, 4, 0x49, 9, 2, 0x42, 1, 0x41].contains(&raw[1])
        || raw[19] != raw[..19].iter().fold(0u8, |s, b| s.wrapping_add(*b))
    {
        return Err("HID 报告长度、命令或校验和无效".into());
    }
    let matrix = [1, 0x41].contains(&raw[1]);
    let limit = if matrix || raw[1] == 9 {
        37
    } else if raw[1] == 0x49 {
        35
    } else if [2, 0x42].contains(&raw[1]) {
        28
    } else {
        15
    };
    if raw[2] == 0
        || raw[2] > limit
        || raw[3] >= raw[2]
        || raw[4] & 15 > 14
        || raw[4] >> 4 > if matrix { 3 } else { 2 }
        || ([0x49, 9, 2, 0x42].contains(&raw[1]) && raw[4] >> 4 != 0)
    {
        return Err("HID 报告分包或格式无效".into());
    }
    let n = (raw[4] & 15) as usize;
    if raw[5 + n..19].iter().any(|v| *v != 0) {
        return Err("HID 填充区必须为零".into());
    }
    if [0x44, 0x49, 0x42, 0x41].contains(&raw[1]) && (raw[2] != 1 || raw[3] != 0 || n != 0) {
        return Err("读取请求格式无效".into());
    }
    if [4, 9, 2, 1].contains(&raw[1]) {
        let bytes = match raw[1] {
            4 => 128,
            9 | 1 => 512,
            2 => {
                if raw[2] == 27 {
                    378
                } else {
                    384
                }
            }
            _ => unreachable!(),
        };
        let count = (bytes as usize).div_ceil(14);
        let expected = if raw[3] as usize + 1 == count {
            bytes as usize - (count - 1) * 14
        } else {
            14
        };
        if raw[2] as usize != count || n != expected {
            return Err("写入块长度或分包长度无效".into());
        }
    }
    Ok(raw.try_into().unwrap())
}

pub fn packet(payload: &[u8], format: u8, total: u8, index: u8) -> [u8; 20] {
    assert!(
        payload.len() <= 14
            && (1..=15).contains(&total)
            && index < total
            && [1, 2].contains(&format)
    );
    let mut raw = [0u8; 20];
    raw[..5].copy_from_slice(&[
        0x13,
        0x88,
        total,
        index,
        (format << 4) | payload.len() as u8,
    ]);
    raw[5..5 + payload.len()].copy_from_slice(payload);
    raw[19] = raw[..19].iter().fold(0u8, |s, b| s.wrapping_add(*b));
    raw
}
fn distance(a: [u8; 3], b: [u8; 3]) -> i32 {
    (0..3).map(|i| (a[i] as i32 - b[i] as i32).pow(2)).sum()
}

pub struct StreamFrame {
    pub reports: Vec<[u8; 20]>,
    pub colors: Vec<[u8; 3]>,
}

// Reserve room for a multicolor palette, then retain the brightest spectrum
// points. Every report belongs to a complete frame fitting the current budget.
pub fn encode_bounded_frame(source: &[[u8; 3]], max_packets: usize) -> StreamFrame {
    assert_eq!(source.len(), 87);
    let budget = max_packets.clamp(1, 15) * 14;
    let mut unique: Vec<_> = source.iter().copied().filter(|c| *c != [0; 3]).collect();
    unique.sort(); unique.dedup();
    let palette_size = (budget / 10).clamp(2, 8).min(unique.len().max(1));
    let max_lit = budget.saturating_sub(palette_size * 4);
    let mut indices: Vec<_> = (0..87).filter(|&i| source[i] != [0; 3]).collect();
    indices.sort_by_key(|&i| std::cmp::Reverse(source[i].iter().map(|&v| v as u32 * v as u32).sum::<u32>()));
    indices.truncate(max_lit);
    let mut limited = vec![[0; 3]; 87];
    for i in indices { limited[i] = source[i]; }
    let full = encode(source, 8);
    let reports = if full.len() <= max_packets.clamp(1, 15) { full } else { encode(&limited, palette_size) };
    let payload: Vec<_> = reports.iter().flat_map(|r| r[5..5 + (r[4] & 15) as usize].iter().copied()).collect();
    let mut colors = vec![[0; 3]; 87];
    if reports[0][4] >> 4 == 2 {
        colors.fill(payload[..3].try_into().unwrap());
    } else {
        let keys = crate::model::keys();
        let mut offset = 0;
        while offset < payload.len() {
            let rgb = payload[offset..offset + 3].try_into().unwrap();
            let count = payload[offset + 3] as usize;
            for &id in &payload[offset + 4..offset + 4 + count] {
                colors[keys.iter().position(|k| k.index == id).unwrap()] = rgb;
            }
            offset += 4 + count;
        }
    }
    StreamFrame { reports, colors }
}

#[cfg(test)]
mod stream_frame_tests {
    use super::*;
    #[test]
    fn all_budgets_produce_complete_frames_with_valid_checksums() {
        for budget in 1..=15 {
            for shift in 0..32u16 {
                let source: Vec<_> = (0..87u16).map(|i| [(i*3+shift) as u8,(i*7) as u8,(255-i) as u8]).collect();
                let frame = encode_bounded_frame(&source, budget);
                assert!(frame.reports.len() <= budget);
                for (i, report) in frame.reports.iter().enumerate() {
                    assert_eq!(report[2] as usize, frame.reports.len());
                    assert_eq!(report[3] as usize, i);
                    assert!(validate_report(report).is_ok());
                }
                assert_eq!(frame.colors.len(),87);
            }
        }
    }
    #[test]
    fn larger_budget_keeps_more_than_two_colors() {
        let source: Vec<_> = (0..87u16).map(|i| [(i*3) as u8,(i*7) as u8,(255-i) as u8]).collect();
        let frame = encode_bounded_frame(&source, 12);
        let mut colors=frame.colors.clone();colors.sort();colors.dedup();
        assert!(colors.iter().filter(|c| **c != [0;3]).count() > 2);
        assert!(frame.reports.len() > 1);
    }
    #[test]
    fn silence_is_one_black_packet_at_every_budget() {
        for budget in 1..=15 {
            let frame=encode_bounded_frame(&vec![[0;3];87],budget);
            assert_eq!(frame.reports.len(),1);
            assert!(frame.colors.iter().all(|c| *c == [0;3]));
        }
    }
}

pub fn encode(colors: &[[u8; 3]], max_colors: usize) -> Vec<[u8; 20]> {
    assert_eq!(colors.len(), 87);
    assert!((1..=24).contains(&max_colors));
    let keys = crate::model::keys();
    let mut unique: Vec<_> = colors.iter().copied().filter(|c| *c != [0; 3]).collect();
    unique.sort();
    unique.dedup();
    let (payload, format) = if unique.is_empty() {
        (vec![0; 3], 2)
    } else if unique.len() == 1 && colors.iter().all(|c| *c != [0; 3]) {
        (unique[0].to_vec(), 2)
    } else {
        let mut palette = unique.clone();
        if unique.len() > max_colors {
            palette = vec![*unique
                .iter()
                .rev()
                .max_by_key(|c| distance(**c, [0; 3]))
                .unwrap()];
            while palette.len() < max_colors {
                palette.push(
                    *unique
                        .iter()
                        .rev()
                        .max_by_key(|c| palette.iter().map(|p| distance(**c, *p)).min().unwrap())
                        .unwrap(),
                );
            }
        }
        let mut groups: Vec<([u8; 3], Vec<u8>)> = Vec::new();
        for (key, color) in keys.iter().zip(colors) {
            if *color == [0; 3] {
                continue;
            }
            let color = *palette
                .iter()
                .min_by_key(|p| distance(**p, *color))
                .unwrap();
            if let Some(g) = groups.iter_mut().find(|g| g.0 == color) {
                g.1.push(key.index);
            } else {
                groups.push((color, vec![key.index]));
            }
        }
        let mut payload = Vec::new();
        for (c, indices) in groups {
            payload.extend(c);
            payload.push(indices.len() as u8);
            payload.extend(indices);
        }
        assert!(payload.len() <= 200);
        (payload, 1)
    };
    let total = payload.len().div_ceil(14) as u8;
    payload
        .chunks(14)
        .enumerate()
        .map(|(i, p)| packet(p, format, total, i as u8))
        .collect()
}
pub fn frame_colors(frame: &Frame, keys: &[Key]) -> Vec<[u8; 3]> {
    keys.iter()
        .map(|k| {
            frame.colors[&k.index]
                .map(|c| ((c as f32 * frame.brightness as f32 / 100.).round()) as u8)
        })
        .collect()
}

