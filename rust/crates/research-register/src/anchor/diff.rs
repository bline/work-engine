//! Native byte-line differ. Line terminators are part of line identity.
#![forbid(unsafe_code)]
pub type Hunk = (usize, usize, usize, usize);

fn lines(data: &[u8]) -> Vec<&[u8]> {
    data.split_inclusive(|&b| b == b'\n').collect()
}

fn mark(a: &[&[u8]], b: &[&[u8]], ca: &mut [bool], cb: &mut [bool]) {
    let mut start = 0;
    while start < a.len().min(b.len()) && a[start] == b[start] {
        start += 1;
    }
    let (mut n, mut m) = (a.len(), b.len());
    while n > start && m > start && a[n - 1] == b[m - 1] {
        n -= 1;
        m -= 1;
    }
    let (a, b, ca, cb) = (
        &a[start..n],
        &b[start..m],
        &mut ca[start..n],
        &mut cb[start..m],
    );
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        cb.fill(true);
        return;
    }
    if m == 0 {
        ca.fill(true);
        return;
    }
    // Meet-in-the-middle shortest edit search, with linear frontier storage.
    let size = n + m + 3;
    let off = m as isize + 1;
    let idx = |k: isize| (k + off) as usize;
    let delta = n as isize - m as isize;
    let mut f = vec![-1isize; size];
    let mut r = vec![n as isize + 1; size];
    f[idx(0)] = 0;
    r[idx(delta)] = n as isize;
    let mut split = None;
    'search: for d in 1..=n + m {
        let d = d as isize;
        for k in (-d..=d).rev().step_by(2) {
            if k < -(m as isize) || k > n as isize {
                continue;
            }
            let mut x = if k == -d {
                f[idx(k + 1)]
            } else if k == d {
                f[idx(k - 1)] + 1
            } else {
                (f[idx(k - 1)] + 1).max(f[idx(k + 1)])
            };
            let mut y = x - k;
            while x >= 0
                && y >= 0
                && x < n as isize
                && y < m as isize
                && a[x as usize] == b[y as usize]
            {
                x += 1;
                y += 1;
            }
            f[idx(k)] = x;
            if delta % 2 != 0 && (k - delta).abs() < d && x >= r[idx(k)] {
                split = Some((x as usize, y as usize));
                break 'search;
            }
        }
        for k in (delta - d..=delta + d).rev().step_by(2) {
            if k < -(m as isize) || k > n as isize {
                continue;
            }
            let mut x = if k == delta - d {
                r[idx(k + 1)] - 1
            } else if k == delta + d {
                r[idx(k - 1)]
            } else {
                r[idx(k - 1)].min(r[idx(k + 1)] - 1)
            };
            let mut y = x - k;
            while x > 0
                && y > 0
                && x <= n as isize
                && y <= m as isize
                && a[x as usize - 1] == b[y as usize - 1]
            {
                x -= 1;
                y -= 1;
            }
            r[idx(k)] = x;
            if delta % 2 == 0 && k.abs() <= d && x <= f[idx(k)] {
                split = Some((x as usize, y as usize));
                break 'search;
            }
        }
    }
    if let Some((x, y)) =
        split.filter(|&(x, y)| x <= n && y <= m && (x + y) > 0 && (x < n || y < m))
    {
        let (cl, cr) = ca.split_at_mut(x);
        let (dl, dr) = cb.split_at_mut(y);
        mark(&a[..x], &b[..y], cl, dl);
        mark(&a[x..], &b[y..], cr, dr);
        return;
    }
    ca.fill(true);
    cb.fill(true);
}

// Equivalent placements of a run of edits are moved towards the end.
fn compact(a: &[&[u8]], changed: &mut [bool], other: &[bool]) {
    let mut boundaries = vec![false; other.len() + 1];
    let mut r = 0;
    for &c in other {
        if c {
            boundaries[r] = true;
        } else {
            r += 1;
        }
    }
    let mut rank = 0;
    let mut i = 0;
    while i < changed.len() {
        if !changed[i] {
            rank += 1;
            i += 1;
            continue;
        }
        let mut s = i;
        while i < changed.len() && changed[i] {
            i += 1;
        }
        // Merging a run can expose additional equivalent placements.
        loop {
            let initial_len = i - s;
            // Explore all equivalent boundaries, merging adjacent edit runs.
            while s > 0 && a[s - 1] == a[i - 1] {
                s -= 1;
                i -= 1;
                rank -= 1;
                changed[s] = true;
                changed[i] = false;
                while s > 0 && changed[s - 1] {
                    s -= 1;
                }
            }
            let mut aligned = if boundaries[rank] { Some((s, i)) } else { None };
            while i < changed.len() && a[s] == a[i] {
                changed[s] = false;
                changed[i] = true;
                s += 1;
                i += 1;
                rank += 1;
                let mut merged = false;
                while i < changed.len() && changed[i] {
                    i += 1;
                    merged = true;
                }
                if merged {
                    aligned = None;
                }
                if boundaries[rank] {
                    aligned = Some((s, i));
                }
            }
            if i - s != initial_len {
                continue;
            }
            if let Some((target, _)) = aligned {
                while s > target {
                    s -= 1;
                    i -= 1;
                    rank -= 1;
                    changed[s] = true;
                    changed[i] = false;
                }
            }
            break;
        }
    }
}

pub fn line_hunks(old: &[u8], new: &[u8]) -> Vec<Hunk> {
    if old == new {
        return Vec::new();
    }
    // Oracle observations show that whole matching 1024-byte tail blocks
    // are excluded before line alignment. Retain any partial boundary line:
    // compaction must not slide edits into the excluded suffix.
    let mut tail = 0;
    while tail + 1024 <= old.len().min(new.len())
        && old[old.len() - tail - 1024..old.len() - tail]
            == new[new.len() - tail - 1024..new.len() - tail]
    {
        tail += 1024;
    }
    // The line containing the cutoff is retained even at an exact line start.
    tail = tail.saturating_sub(1);
    while tail > 0
        && ((tail < old.len() && old[old.len() - tail - 1] != b'\n')
            || (tail < new.len() && new[new.len() - tail - 1] != b'\n'))
    {
        tail -= 1;
    }
    let a = lines(&old[..old.len() - tail]);
    let b = lines(&new[..new.len() - tail]);
    let mut ca = vec![false; a.len()];
    let mut cb = vec![false; b.len()];
    // Lines absent from the opposite input cannot be part of a common path.
    let sa: std::collections::HashSet<_> = a.iter().copied().collect();
    let sb: std::collections::HashSet<_> = b.iter().copied().collect();
    let ia: Vec<_> = (0..a.len()).filter(|&i| sb.contains(a[i])).collect();
    let ib: Vec<_> = (0..b.len()).filter(|&i| sa.contains(b[i])).collect();
    ca.fill(true);
    cb.fill(true);
    let aa: Vec<_> = ia.iter().map(|&i| a[i]).collect();
    let bb: Vec<_> = ib.iter().map(|&i| b[i]).collect();
    let mut da = vec![false; aa.len()];
    let mut db = vec![false; bb.len()];
    mark(&aa, &bb, &mut da, &mut db);
    for (i, c) in ia.into_iter().zip(da) {
        ca[i] = c;
    }
    for (i, c) in ib.into_iter().zip(db) {
        cb[i] = c;
    }
    compact(&a, &mut ca, &cb);
    compact(&b, &mut cb, &ca);
    let (mut i, mut j) = (0, 0);
    let mut result = Vec::new();
    while i < a.len() || j < b.len() {
        if (i < a.len() && ca[i]) || (j < b.len() && cb[j]) {
            let (s, t) = (i, j);
            while i < a.len() && ca[i] {
                i += 1;
            }
            while j < b.len() && cb[j] {
                j += 1;
            }
            result.push((
                if i == s { s } else { s + 1 },
                i - s,
                if j == t { t } else { t + 1 },
                j - t,
            ));
        } else {
            i += 1;
            j += 1;
        }
    }
    result
}
