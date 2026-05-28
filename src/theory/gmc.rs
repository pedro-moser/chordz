use crate::theory::notes::PC_NAMES;
use crate::theory::scales::Scale;

pub struct TriadPairSet {
    pub label: &'static str,
    pub indices: ([usize; 3], [usize; 3]),
}

pub const PAIRS: [TriadPairSet; 10] = [
    TriadPairSet { label: "T/T", indices: ([0, 2, 4], [1, 3, 5]) },
    TriadPairSet { label: "T/7no5", indices: ([0, 3, 5], [1, 2, 4]) },
    TriadPairSet { label: "T/7no3", indices: ([0, 2, 5], [1, 3, 4]) },
    TriadPairSet { label: "Sus/Sus", indices: ([0, 3, 4], [1, 2, 5]) },
    TriadPairSet { label: "Sus/7no5", indices: ([0, 1, 4], [2, 3, 5]) },
    TriadPairSet { label: "Sus/7no3", indices: ([1, 4, 5], [0, 2, 3]) },
    TriadPairSet { label: "Clus/Clus", indices: ([0, 1, 2], [3, 4, 5]) },
    TriadPairSet { label: "Clus/7no5", indices: ([1, 2, 3], [0, 4, 5]) },
    TriadPairSet { label: "Clus/7no3", indices: ([2, 3, 4], [0, 1, 5]) },
    TriadPairSet { label: "7no5/7no3", indices: ([2, 4, 5], [0, 1, 3]) },
];

pub fn resolve_pair(root_pc: u8, scale: &Scale, pair: &TriadPairSet) -> ([u8; 3], [u8; 3]) {
    let tones = scale.non_root_semitones().map(|s| (root_pc + s) % 12);
    let a = [
        tones[pair.indices.0[0]],
        tones[pair.indices.0[1]],
        tones[pair.indices.0[2]],
    ];
    let b = [
        tones[pair.indices.1[0]],
        tones[pair.indices.1[1]],
        tones[pair.indices.1[2]],
    ];
    (a, b)
}

pub fn pair_display(root_pc: u8, scale: &Scale, pair: &TriadPairSet) -> String {
    let tones = scale.non_root_semitones();
    let intervals_a: Vec<&str> = pair.indices.0.iter().map(|&i| scale.interval_name(tones[i])).collect();
    let intervals_b: Vec<&str> = pair.indices.1.iter().map(|&i| scale.interval_name(tones[i])).collect();
    let (pcs_a, pcs_b) = resolve_pair(root_pc, scale, pair);
    let notes_a: Vec<&str> = pcs_a.iter().map(|&pc| PC_NAMES[pc as usize]).collect();
    let notes_b: Vec<&str> = pcs_b.iter().map(|&pc| PC_NAMES[pc as usize]).collect();
    format!(
        "({}) + ({}) = {} + {}",
        intervals_a.join(" "),
        intervals_b.join(" "),
        notes_a.join(" "),
        notes_b.join(" "),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_pairs_cover_all_combinations() {
        for pair in &PAIRS {
            let mut all_indices: Vec<usize> = pair.indices.0.to_vec();
            all_indices.extend_from_slice(&pair.indices.1);
            all_indices.sort();
            assert_eq!(all_indices, vec![0, 1, 2, 3, 4, 5], "pair {} missing indices", pair.label);
        }
    }

    #[test]
    fn no_duplicate_pairs() {
        for i in 0..PAIRS.len() {
            for j in (i + 1)..PAIRS.len() {
                let mut a_i: Vec<usize> = PAIRS[i].indices.0.to_vec();
                a_i.sort();
                let mut a_j: Vec<usize> = PAIRS[j].indices.0.to_vec();
                a_j.sort();
                let mut b_j: Vec<usize> = PAIRS[j].indices.1.to_vec();
                b_j.sort();
                assert!(a_i != a_j && a_i != b_j, "duplicate pair at {} and {}", i, j);
            }
        }
    }

    #[test]
    fn resolve_c_dorian_t_t() {
        let dorian = Scale::ALL.iter().find(|s| s.name == "Dorian").unwrap();
        let (a, b) = resolve_pair(0, dorian, &PAIRS[0]);
        assert_eq!(a, [2, 5, 9]); // D, F, A
        assert_eq!(b, [3, 7, 10]); // Eb, G, Bb
    }

    #[test]
    fn resolve_c_ionian_t_t() {
        let ionian = &Scale::ALL[0];
        let (a, b) = resolve_pair(0, ionian, &PAIRS[0]);
        assert_eq!(a, [2, 5, 9]); // D, F, A
        assert_eq!(b, [4, 7, 11]); // E, G, B
    }

    #[test]
    fn pair_display_shows_intervals_and_notes() {
        let dorian = Scale::ALL.iter().find(|s| s.name == "Dorian").unwrap();
        let display = pair_display(0, dorian, &PAIRS[0]);
        assert!(display.contains("2 4 6"), "expected intervals 2 4 6, got: {}", display);
        assert!(display.contains("b3 5 b7"), "expected intervals b3 5 b7, got: {}", display);
    }

    #[test]
    fn verify_major_scale_all_modes_t_t() {
        // From Pedro's spreadsheet: Major scale, T/T row
        // All columns use C as root
        let expected: &[(&str, [u8; 3], [u8; 3])] = &[
            // C Ionian: D,F,A + E,G,B
            ("Ionian", [2, 5, 9], [4, 7, 11]),
            // C Dorian: D,F,A + Eb,G,Bb
            ("Dorian", [2, 5, 9], [3, 7, 10]),
            // C Phrygian: Db,F,Ab + Eb,G,Bb
            ("Phrygian", [1, 5, 8], [3, 7, 10]),
            // C Lydian: D,F#,A + E,G,B
            ("Lydian", [2, 6, 9], [4, 7, 11]),
            // C Mixolydian: D,F,A + E,G,Bb
            ("Mixolydian", [2, 5, 9], [4, 7, 10]),
            // C Aeolian: D,F,Ab + Eb,G,Bb
            ("Aeolian", [2, 5, 8], [3, 7, 10]),
            // C Locrian: Db,F,Ab + Eb,Gb,Bb
            ("Locrian", [1, 5, 8], [3, 6, 10]),
        ];

        for (name, exp_a, exp_b) in expected {
            let scale = Scale::ALL.iter().find(|s| s.name == *name).unwrap();
            let (a, b) = resolve_pair(0, scale, &PAIRS[0]);
            assert_eq!(a, *exp_a, "T/T group A mismatch for C {}", name);
            assert_eq!(b, *exp_b, "T/T group B mismatch for C {}", name);
        }
    }

    #[test]
    fn verify_melodic_minor_t_t() {
        // From spreadsheet: Melodic Minor, T/T row, all C root
        let expected: &[(&str, [u8; 3], [u8; 3])] = &[
            // CmΔ7: D,F,A + Eb,G,B
            ("Melodic Minor", [2, 5, 9], [3, 7, 11]),
            // C Altered: Db,Fb(4),Ab + Eb,Gb,Bb
            ("Altered", [1, 4, 8], [3, 6, 10]),
        ];

        for (name, exp_a, exp_b) in expected {
            let scale = Scale::ALL.iter().find(|s| s.name == *name).unwrap();
            let (a, b) = resolve_pair(0, scale, &PAIRS[0]);
            assert_eq!(a, *exp_a, "T/T group A mismatch for C {}", name);
            assert_eq!(b, *exp_b, "T/T group B mismatch for C {}", name);
        }
    }

    #[test]
    fn verify_dorian_clus_clus() {
        // Spreadsheet: C Dorian, Clus/Clus: F,Eb,D + Bb,A,G
        // sorted by PC: D(2),Eb(3),F(5) + G(7),A(9),Bb(10)
        let dorian = Scale::ALL.iter().find(|s| s.name == "Dorian").unwrap();
        let (a, b) = resolve_pair(0, dorian, &PAIRS[6]); // Clus/Clus
        // indices [0,1,2] + [3,4,5] on [2,3,5,7,9,10]
        assert_eq!(a, [2, 3, 5]); // D, Eb, F
        assert_eq!(b, [7, 9, 10]); // G, A, Bb
    }
}
