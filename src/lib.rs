#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]

use std::fmt::Write;
use std::process::Command;
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};

#[derive(Debug, PartialEq)]
pub struct Filesystem {
    name: String,
    pools: Vec<Pool>,
}

impl Filesystem {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn pools(&self) -> &[Pool] {
        &self.pools
    }
}

/// Runs `mmlspool` on all given filesystems, and returns the parsed output.
///
/// # Errors
///
/// Returns an error if running `mmlspool` fails or if parsing its output
/// fails.
pub fn run_all<S>(fs_names: &[S]) -> Result<Vec<Filesystem>>
where
    S: AsRef<str>,
{
    let mut filesystems = Vec::with_capacity(fs_names.len());

    for fs in fs_names {
        let filesystem = run(fs.as_ref())?;
        filesystems.push(filesystem);
    }

    Ok(filesystems)
}

/// Runs `mmlspool` on the given filesystem, and returns the parsed output.
///
/// # Errors
///
/// Returns an error if running `mmlspool` fails or if parsing its output
/// fails.
pub fn run(fs_name: &str) -> Result<Filesystem> {
    let mut cmd = Command::new("mmlspool");
    cmd.arg(&fs_name);

    let output = cmd
        .output()
        .with_context(|| format!("error running: {:?}", cmd))?;

    if output.status.success() {
        let output = String::from_utf8(output.stdout).with_context(|| {
            format!("parsing {:?} command output to UTF8", cmd)
        })?;

        let pools = parse_mmlspool_output(&output)
            .with_context(|| "parsing pools to internal data")?;

        Ok(Filesystem {
            name: fs_name.into(),
            pools,
        })
    } else {
        Err(anyhow!("error running: {:?}", cmd))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoolSize {
    total_kb: u64,
    free_kb: u64,
}

impl PoolSize {
    #[must_use]
    pub const fn total_kb(&self) -> u64 {
        self.total_kb
    }

    #[must_use]
    pub const fn free_kb(&self) -> u64 {
        self.free_kb
    }

    #[must_use]
    pub const fn used_percent(&self) -> u64 {
        let used_kb = self.total_kb - self.free_kb;
        let x = used_kb * 100;
        let y = self.total_kb;

        x / y
    }
}

#[derive(Debug, PartialEq)]
pub struct Pool {
    name: String,
    data: Option<PoolSize>,
    meta: Option<PoolSize>,
}

impl Pool {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn data(&self) -> Option<&PoolSize> {
        self.data.as_ref()
    }

    #[must_use]
    pub const fn meta(&self) -> Option<&PoolSize> {
        self.meta.as_ref()
    }
}

impl FromStr for Pool {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let tokens = s
            .split(' ')
            .filter(|token| !token.is_empty())
            .collect::<Vec<_>>();

        let name = tokens[0].into();

        let data = if tokens[4] == "yes" {
            let total_kb = tokens[6].parse::<u64>().with_context(|| {
                format!("parsing data totalkb token {} to u64", tokens[6])
            })?;

            let free_kb = tokens[7].parse::<u64>().with_context(|| {
                format!("parsing data freekb token {} to u64", tokens[7])
            })?;

            Some(PoolSize { total_kb, free_kb })
        } else {
            None
        };

        let meta = if tokens[5] == "yes" {
            let (total_kb_token_id, free_kb_token_id) =
                if tokens[8] == "(" { (10, 11) } else { (9, 10) };

            let total_kb = tokens[total_kb_token_id]
                .parse::<u64>()
                .with_context(|| {
                    format!(
                        "parsing meta totalkb token {} to u64",
                        tokens[total_kb_token_id]
                    )
                })?;

            let free_kb = tokens[free_kb_token_id]
                .parse::<u64>()
                .with_context(|| {
                    format!(
                        "parsing meta freekb token {} to u64",
                        tokens[free_kb_token_id]
                    )
                })?;

            Some(PoolSize { total_kb, free_kb })
        } else {
            None
        };

        if data.is_none() && meta.is_none() {
            Err(anyhow!("pool {} contains neither data nor metadata", name))
        } else {
            Ok(Self { name, data, meta })
        }
    }
}

fn parse_mmlspool_output(s: &str) -> Result<Vec<Pool>> {
    let mut pools = Vec::with_capacity(16);

    for line in s.lines().skip(2) {
        let pool = line
            .parse()
            .with_context(|| format!("parsing pool line: {}", line))?;

        pools.push(pool);
    }

    Ok(pools)
}

/// Converts filesystems to prometheus metric format.
///
/// # Errors
///
/// This functions uses [`writeln`] to write to the output [`String`] that is
/// returned. It can only fail if any of these [`writeln`] fails.
pub fn to_prom(filesystems: &[Filesystem]) -> Result<String> {
    let mut output = String::default();

    writeln!(
        output,
        "# HELP gpfs_fs_pool_total_kbytes GPFS pool size in kilobytes."
    )?;
    writeln!(output, "# TYPE gpfs_fs_pool_total_kbytes gauge")?;

    for fs in filesystems {
        for pool in &fs.pools {
            if let Some(size) = &pool.data {
                writeln!(
                    output,
                    "gpfs_fs_pool_total_kbytes{{fs=\"{}\",pool=\"{}\",type=\"data\"}} {}",
                    fs.name,
                    pool.name,
                    size.total_kb
                )?;
            }

            if let Some(size) = &pool.meta {
                writeln!(
                    output,
                    "gpfs_fs_pool_total_kbytes{{fs=\"{}\",pool=\"{}\",type=\"meta\"}} {}",
                    fs.name,
                    pool.name,
                    size.total_kb
                )?;
            }
        }
    }

    writeln!(output)?;
    writeln!(
        output,
        "# HELP gpfs_fs_pool_free_kbytes GPFS pool free kilobytes."
    )?;
    writeln!(output, "# TYPE gpfs_fs_pool_free_kbytes gauge")?;

    for fs in filesystems {
        for pool in &fs.pools {
            if let Some(size) = &pool.data {
                writeln!(
                    output,
                    "gpfs_fs_pool_free_kbytes{{fs=\"{}\",pool=\"{}\",type=\"data\"}} {}",
                    fs.name,
                    pool.name,
                    size.free_kb
                )?;
            }

            if let Some(size) = &pool.meta {
                writeln!(
                    output,
                    "gpfs_fs_pool_free_kbytes{{fs=\"{}\",pool=\"{}\",type=\"meta\"}} {}",
                    fs.name,
                    pool.name,
                    size.free_kb
                )?;
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let s = concat!(
            "Storage pools in file system at '/gpfs1':\n",
            "Name                    Id   BlkSize Data Meta Total Data in (KB)   Free Data in (KB)   Total Meta in (KB)    Free Meta in (KB)\n",
            "system                   0      8 MB   no  yes              0              0 (  0%)    25004867584     9798959104 ( 39%)\n",
            "nvme                 65537      8 MB  yes   no   162531639296   114505474048 ( 70%)              0              0 (  0%)\n",
            "nlsas                65538      8 MB  yes   no  1997953957888  1981410271232 ( 99%)              0              0 (  0%)\n",
            "dangerzone           65538      8 MB  yes  yes             42             42 (100%)             42             42 (100%)\n",
        );

        let prom = concat!(
            "# HELP gpfs_fs_pool_total_kbytes GPFS pool size in kilobytes.\n",
            "# TYPE gpfs_fs_pool_total_kbytes gauge\n",
            "gpfs_fs_pool_total_kbytes{fs=\"gpfs1\",pool=\"system\",type=\"meta\"} 25004867584\n",
            "gpfs_fs_pool_total_kbytes{fs=\"gpfs1\",pool=\"nvme\",type=\"data\"} 162531639296\n",
            "gpfs_fs_pool_total_kbytes{fs=\"gpfs1\",pool=\"nlsas\",type=\"data\"} 1997953957888\n",
            "gpfs_fs_pool_total_kbytes{fs=\"gpfs1\",pool=\"dangerzone\",type=\"data\"} 42\n",
            "gpfs_fs_pool_total_kbytes{fs=\"gpfs1\",pool=\"dangerzone\",type=\"meta\"} 42\n",
            "\n",
            "# HELP gpfs_fs_pool_free_kbytes GPFS pool free kilobytes.\n",
            "# TYPE gpfs_fs_pool_free_kbytes gauge\n",
            "gpfs_fs_pool_free_kbytes{fs=\"gpfs1\",pool=\"system\",type=\"meta\"} 9798959104\n",
            "gpfs_fs_pool_free_kbytes{fs=\"gpfs1\",pool=\"nvme\",type=\"data\"} 114505474048\n",
            "gpfs_fs_pool_free_kbytes{fs=\"gpfs1\",pool=\"nlsas\",type=\"data\"} 1981410271232\n",
            "gpfs_fs_pool_free_kbytes{fs=\"gpfs1\",pool=\"dangerzone\",type=\"data\"} 42\n",
            "gpfs_fs_pool_free_kbytes{fs=\"gpfs1\",pool=\"dangerzone\",type=\"meta\"} 42\n",
        );

        let pools = parse_mmlspool_output(s).unwrap();
        assert_eq!(pools.len(), 4);

        assert_eq!(
            pools[0],
            Pool {
                name: "system".into(),
                data: None,
                meta: Some(PoolSize {
                    total_kb: 25_004_867_584,
                    free_kb: 9_798_959_104,
                }),
            }
        );

        assert_eq!(
            &pools[1],
            &Pool {
                name: "nvme".into(),
                data: Some(PoolSize {
                    total_kb: 162_531_639_296,
                    free_kb: 114_505_474_048,
                }),
                meta: None,
            }
        );

        assert_eq!(pools[1].data.unwrap().used_percent(), 29);

        assert_eq!(
            pools[2],
            Pool {
                name: "nlsas".into(),
                data: Some(PoolSize {
                    total_kb: 1_997_953_957_888,
                    free_kb: 1_981_410_271_232,
                }),
                meta: None,
            }
        );

        assert_eq!(
            pools[3],
            Pool {
                name: "dangerzone".into(),
                data: Some(PoolSize {
                    total_kb: 42,
                    free_kb: 42,
                }),
                meta: Some(PoolSize {
                    total_kb: 42,
                    free_kb: 42,
                }),
            }
        );

        let fs = Filesystem {
            name: "gpfs1".into(),
            pools,
        };

        assert_eq!(to_prom(&[fs]).unwrap(), prom);
    }
}
