//! A line diff for `/diff`: rows marked ' ', '-', or '+', grouped into
//! hunks with three lines of context and `⋯` between them.

/// Context lines kept around each change.
const CONTEXT: usize = 3;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Op {
    Same,
    Gone,
    Came,
}

/// The rows of a diff from `old` to `new`, and the added and removed counts.
pub fn hunks(old: &str, new: &str) -> (Vec<String>, usize, usize) {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    // The common start and end cost nothing; only the middle is compared.
    let start = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let end = a[start..].iter().rev().zip(b[start..].iter().rev()).take_while(|(x, y)| x == y).count();
    let (mid_a, mid_b) = (&a[start..a.len() - end], &b[start..b.len() - end]);
    let mut ops: Vec<(Op, &str)> = a[..start].iter().map(|l| (Op::Same, *l)).collect();
    // ponytail: O(n·m) on the changed middle; past four million cells it is
    // shown as all removed then all added. A Myers diff if that bites.
    if mid_a.len() * mid_b.len() <= 4_000_000 {
        let mut dp = vec![vec![0u32; mid_b.len() + 1]; mid_a.len() + 1];
        for i in (0..mid_a.len()).rev() {
            for j in (0..mid_b.len()).rev() {
                dp[i][j] = if mid_a[i] == mid_b[j] { dp[i + 1][j + 1] + 1 } else { dp[i + 1][j].max(dp[i][j + 1]) };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < mid_a.len() || j < mid_b.len() {
            if i < mid_a.len() && j < mid_b.len() && mid_a[i] == mid_b[j] {
                ops.push((Op::Same, mid_a[i]));
                i += 1;
                j += 1;
            } else if i < mid_a.len() && (j == mid_b.len() || dp[i + 1][j] >= dp[i][j + 1]) {
                ops.push((Op::Gone, mid_a[i]));
                i += 1;
            } else {
                ops.push((Op::Came, mid_b[j]));
                j += 1;
            }
        }
    } else {
        ops.extend(mid_a.iter().map(|l| (Op::Gone, *l)));
        ops.extend(mid_b.iter().map(|l| (Op::Came, *l)));
    }
    ops.extend(a[a.len() - end..].iter().map(|l| (Op::Same, *l)));

    let changed: Vec<usize> = (0..ops.len()).filter(|&i| ops[i].0 != Op::Same).collect();
    let (adds, dels) = (ops.iter().filter(|o| o.0 == Op::Came).count(), ops.iter().filter(|o| o.0 == Op::Gone).count());
    let mut rows = Vec::new();
    let mut shown_to = 0;
    for (k, &c) in changed.iter().enumerate() {
        let from = c.saturating_sub(CONTEXT).max(shown_to);
        if k > 0 && from > shown_to {
            rows.push("⋯".to_string());
        }
        let next = changed.get(k + 1).copied();
        // Run on to the next change when their contexts touch.
        let to = match next {
            Some(n) if n <= c + 2 * CONTEXT + 1 => n,
            _ => (c + 1 + CONTEXT).min(ops.len()),
        };
        for (op, line) in &ops[from..to] {
            let sign = match op {
                Op::Same => ' ',
                Op::Gone => '-',
                Op::Came => '+',
            };
            rows.push(format!("{sign}{line}"));
        }
        shown_to = to;
    }
    (rows, adds, dels)
}

/// Pair the removed and added lines of one change, in order, for a side-by-side
/// view. Unpaired lines get an empty cell on the other side.
pub fn pair_change(removed: &[String], added: &[String]) -> Vec<(Option<String>, Option<String>)> {
    let rows = removed.len().max(added.len());
    (0..rows)
        .map(|i| (removed.get(i).cloned(), added.get(i).cloned()))
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_change_pairs_lines_in_order_and_leaves_gaps() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let rows = super::pair_change(&s(&["a", "b"]), &s(&["x"]));
        assert_eq!(rows, vec![(Some("a".into()), Some("x".into())), (Some("b".into()), None)]);
        let rows = super::pair_change(&[], &s(&["y"]));
        assert_eq!(rows, vec![(None, Some("y".into()))]);
        assert!(super::pair_change(&[], &[]).is_empty());
    }

    use super::*;

    fn lines(n: usize) -> String {
        (1..=n).map(|i| format!("line {i}\n")).collect()
    }

    #[test]
    fn a_changed_line_shows_with_its_context() {
        let old = lines(10);
        let new = old.replace("line 5\n", "line five\n");
        let (rows, adds, dels) = hunks(&old, &new);
        assert_eq!((adds, dels), (1, 1));
        assert_eq!(rows, vec![" line 2", " line 3", " line 4", "-line 5", "+line five", " line 6", " line 7", " line 8"]);
    }

    #[test]
    fn far_apart_changes_are_two_hunks() {
        let old = lines(30);
        let new = old.replace("line 3\n", "line three\n").replace("line 27\n", "line twenty-seven\n");
        let (rows, _, _) = hunks(&old, &new);
        assert_eq!(rows.iter().filter(|r| *r == "⋯").count(), 1, "{rows:?}");
        assert!(rows.contains(&"+line three".to_string()) && rows.contains(&"+line twenty-seven".to_string()));
    }

    #[test]
    fn a_new_file_is_all_added_and_no_change_is_nothing() {
        let (rows, adds, dels) = hunks("", "one\ntwo\n");
        assert_eq!((rows, adds, dels), (vec!["+one".to_string(), "+two".to_string()], 2, 0));
        assert_eq!(hunks("same\n", "same\n"), (Vec::new(), 0, 0));
    }

    #[test]
    fn insertions_keep_their_place() {
        let (rows, adds, dels) = hunks("a\nb\nc\n", "a\nb\nnew\nc\n");
        assert_eq!((adds, dels), (1, 0));
        assert_eq!(rows, vec![" a", " b", "+new", " c"]);
    }
}
