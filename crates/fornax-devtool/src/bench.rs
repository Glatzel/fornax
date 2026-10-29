use strum::AsRefStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, AsRefStr)]
pub enum BenchSampleBits {
    #[strum(serialize = "f32")]
    F32,
    #[strum(serialize = "f64")]
    F64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchSize {
    pub label: String,
    pub dims: (u32, u32), // (width, height)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchParams {
    pub sizes: Vec<BenchSize>,
    pub bits: Vec<BenchSampleBits>,
    pub threads: Vec<usize>,
}

impl BenchParams {
    /// Reads `FORNAX_BENCH_SIZE` (default `1000,2000`),
    /// `FORNAX_BENCH_BITS` (default `32,64`). Thread counts come from the
    /// global rayon pool size.
    pub fn from_env() -> Self {
        let sizes = std::env::var("FORNAX_BENCH_SIZE").unwrap_or_else(|_| "1000,2000".to_owned());
        let bits = std::env::var("FORNAX_BENCH_BITS").unwrap_or_else(|_| "32,64".to_owned());
        Self {
            sizes: Self::parse_sizes(&sizes),
            bits: Self::parse_bits(&bits),
            threads: Self::default_threads(),
        }
    }

    pub fn parse_sizes(s: &str) -> Vec<BenchSize> {
        s.split(',')
            .map(str::trim)
            .map(|short_edge| {
                let short_edge = short_edge.parse::<u32>().unwrap_or_else(|_| {
                    panic!("unknown FORNAX_BENCH_SIZE value {short_edge:?}; expected a positive integer")
                });
                assert!(
                    short_edge > 0,
                    "unknown FORNAX_BENCH_SIZE value {short_edge:?}; expected a positive integer"
                );
                let long_edge = (u64::from(short_edge) * 3).div_ceil(2);
                let width = u32::try_from(long_edge).unwrap_or_else(|_| {
                    panic!("FORNAX_BENCH_SIZE value {short_edge:?} exceeds supported dimensions")
                });
                let pixels = u64::from(width) * u64::from(short_edge);
                BenchSize {
                    label: format!("{:.2}MP", pixels as f64 / 1_000_000.0),
                    dims: (width, short_edge),
                }
            })
            .collect()
    }

    pub fn parse_bits(s: &str) -> Vec<BenchSampleBits> {
        s.split(',')
            .map(str::trim)
            .map(|bits| match bits {
                "32" => BenchSampleBits::F32,
                "64" => BenchSampleBits::F64,
                _ => panic!("unknown FORNAX_BENCH_BITS value {bits:?}; expected 32, 64, or a comma-separated list"),
            })
            .collect()
    }

    /// 1, 2, 4, ... up to the global pool size (always includes the max).
    pub fn default_threads() -> Vec<usize> {
        // GitHub Actions, GitLab CI and most other providers set `CI`.
        // CodSpeed simulation runs under the same environment.
        if std::env::var_os("CI").is_some() {
            return vec![1];
        }

        let max = rayon::current_num_threads();
        let mut v: Vec<usize> = std::iter::successors(Some(1_usize), |&n| n.checked_mul(2))
            .take_while(|&n| n < max)
            .collect();
        v.push(max);
        v
    }

    /// Builder-style override, e.g. `BenchParams::from_env().with_threads([1])`
    /// for CI.
    pub fn with_threads<T>(mut self, threads: T) -> Self
    where
        T: IntoIterator<Item = usize>,
    {
        self.threads = threads.into_iter().collect();
        self
    }

    /// `linear/1.50MP/f32/4T`; the `/{n}T` suffix is dropped when only one
    /// thread count is configured, so names match an unswept baseline.
    pub fn case_name(
        &self,
        prefix: &str,
        size: &BenchSize,
        bits: BenchSampleBits,
        threads: usize,
    ) -> String {
        if self.threads.len() == 1 {
            format!("{prefix}/{}/{}", size.label, bits.as_ref())
        } else {
            format!("{prefix}/{}/{}/{threads}T", size.label, bits.as_ref())
        }
    }

    /// Build one rayon pool per thread count.
    pub fn pools(&self) -> impl Iterator<Item = (usize, rayon::ThreadPool)> + '_ {
        self.threads.iter().map(|&n| {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(n)
                .build()
                .unwrap();
            (n, pool)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_use_three_by_two_dimensions_and_megapixel_labels() {
        let sizes = BenchParams::parse_sizes("1000, 2001");
        assert_eq!(
            sizes,
            vec![
                BenchSize {
                    label: "1.50MP".to_owned(),
                    dims: (1_500, 1_000),
                },
                BenchSize {
                    label: "6.01MP".to_owned(),
                    dims: (3_002, 2_001),
                },
            ]
        );
    }

    #[test]
    fn size_parser_rejects_invalid_values() {
        for value in ["", "1000,,2000", "0", "-1", "invalid", "4294967295"] {
            assert!(
                std::panic::catch_unwind(|| BenchParams::parse_sizes(value)).is_err(),
                "{value:?} should be rejected"
            );
        }
    }

    #[test]
    fn bits_preserve_order_and_reject_unknown_values() {
        assert_eq!(
            BenchParams::parse_bits("64, 32"),
            vec![BenchSampleBits::F64, BenchSampleBits::F32]
        );
        for value in ["", "16", "invalid"] {
            assert!(
                std::panic::catch_unwind(|| BenchParams::parse_bits(value)).is_err(),
                "{value:?} should be rejected"
            );
        }
    }

    #[test]
    fn case_names_only_include_threads_for_multi_thread_sweeps() {
        let size = BenchSize {
            label: "1.50MP".to_owned(),
            dims: (1_500, 1_000),
        };
        let single = BenchParams {
            sizes: vec![size.clone()],
            bits: vec![BenchSampleBits::F32],
            threads: vec![1],
        };
        assert_eq!(
            single.case_name("alpha", &size, BenchSampleBits::F32, 1),
            "alpha/1.50MP/f32"
        );

        let swept = single.with_threads([1, 4]);
        assert_eq!(
            swept.case_name("alpha", &size, BenchSampleBits::F64, 4),
            "alpha/1.50MP/f64/4T"
        );
    }

    #[test]
    fn thread_pools_match_requested_sizes() {
        let params = BenchParams {
            sizes: Vec::new(),
            bits: Vec::new(),
            threads: vec![1, 2],
        };
        let pools = params
            .pools()
            .map(|(configured, pool)| (configured, pool.current_num_threads()))
            .collect::<Vec<_>>();
        assert_eq!(pools, vec![(1, 1), (2, 2)]);
    }
}
