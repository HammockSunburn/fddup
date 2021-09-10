// Copyright (c) 2021 Hammock Sunburn
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the
// "Software"), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
// NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
// LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
// WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

pub trait GetKey<K> {
    fn key(&self) -> K;
    fn bytes_remaining(&self) -> u64;
}

pub struct Work<T> {
    pub work: Vec<T>,
    pub duplicates: Vec<T>,
    pub uniques: Vec<T>,
}

pub fn find_work<T, K>(possible: &mut Vec<T>, desired: usize) -> Work<T>
where
    T: GetKey<K>,
    K: PartialEq + Clone,
{
    let mut last_key: Option<K> = None;
    let mut remaining = desired;
    let mut work = Vec::new();
    let mut duplicates = Vec::new();
    let mut uniques = Vec::new();

    while let Some(item) = possible.pop() {
        let last_key_matches = match &last_key {
            Some(key) => *key == item.key(),
            None => false,
        };

        if remaining == 0 && !last_key_matches {
            possible.push(item);
            break;
        }

        let either_matches = match last_key_matches {
            true => true,
            false => match possible.last() {
                Some(next) => next.key() == item.key(),
                None => false,
            },
        };

        if either_matches {
            last_key = Some(item.key().clone());

            if item.bytes_remaining() == 0 {
                duplicates.push(item);
            } else {
                work.push(item);
                remaining = remaining.saturating_sub(1);
            }
        } else {
            uniques.push(item);
        }
    }

    Work {
        work,
        duplicates,
        uniques,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Eq, PartialEq)]
    struct TestWork {
        id: u32,
        bytes_remaining: u64,
    }

    impl GetKey<u32> for TestWork {
        fn key(&self) -> u32 {
            self.id
        }

        fn bytes_remaining(&self) -> u64 {
            self.bytes_remaining
        }
    }

    fn w_10(id: u32) -> TestWork {
        TestWork {
            id,
            bytes_remaining: 10,
        }
    }

    fn w_0(id: u32) -> TestWork {
        TestWork {
            id,
            bytes_remaining: 0,
        }
    }

    #[test]
    fn find_work_no_dups() {
        let mut input = vec![w_10(1), w_10(2), w_10(3), w_10(4)];
        let w = find_work(&mut input, 4);

        assert_eq!(input, vec![]);
        assert_eq!(w.work, vec![]);
        assert_eq!(w.duplicates, vec![]);
        assert_eq!(w.uniques, vec![w_10(4), w_10(3), w_10(2), w_10(1)])
    }

    #[test]
    fn find_work_only_dups_equal_to_desired() {
        let mut input = vec![w_10(1), w_10(1), w_10(1), w_10(1)];
        let w = find_work(&mut input, 4);

        assert_eq!(input, vec![]);
        assert_eq!(w.work, vec![w_10(1), w_10(1), w_10(1), w_10(1)]);
        assert_eq!(w.duplicates, vec![]);
        assert_eq!(w.uniques, vec![]);
    }

    #[test]
    fn find_work_only_dups_more_than_desired() {
        let mut input = vec![w_10(1), w_10(1), w_10(1), w_10(1)];
        let w = find_work(&mut input, 2);

        assert_eq!(input, vec![]);
        assert_eq!(w.work, vec![w_10(1), w_10(1), w_10(1), w_10(1)]);
        assert_eq!(w.duplicates, vec![]);
        assert_eq!(w.uniques, vec![]);
    }

    #[test]
    fn find_work_only_dups_in_middle_more_than_desired() {
        let mut input = vec![
            w_10(1),
            w_10(2),
            w_10(3),
            w_10(3),
            w_10(3),
            w_10(4),
            w_10(5),
        ];
        let w = find_work(&mut input, 2);

        assert_eq!(input, vec![w_10(1), w_10(2)]);
        assert_eq!(w.work, vec![w_10(3), w_10(3), w_10(3)]);
        assert_eq!(w.duplicates, vec![]);
        assert_eq!(w.uniques, vec![w_10(5), w_10(4)]);
    }

    #[test]
    fn find_work_only_dups_in_middle_less_than_desired() {
        let mut input = vec![
            w_10(1),
            w_10(2),
            w_10(3),
            w_10(3),
            w_10(3),
            w_10(4),
            w_10(5),
        ];
        let w = find_work(&mut input, 4);

        assert_eq!(input, vec![]);
        assert_eq!(w.work, vec![w_10(3), w_10(3), w_10(3)]);
        assert_eq!(w.duplicates, vec![]);
        assert_eq!(w.uniques, vec![w_10(5), w_10(4), w_10(2), w_10(1)]);
    }

    #[test]
    fn find_work_some_work_and_some_duplicates_found() {
        let mut input = vec![w_10(1), w_10(2), w_10(3), w_10(3), w_0(4), w_0(4), w_10(5)];
        let w = find_work(&mut input, 4);

        assert_eq!(input, vec![]);
        assert_eq!(w.work, vec![w_10(3), w_10(3)]);
        assert_eq!(w.duplicates, vec![w_0(4), w_0(4)]);
        assert_eq!(w.uniques, vec![w_10(5), w_10(2), w_10(1)]);
    }

    #[test]
    fn find_work_only_duplicates_found() {
        let mut input = vec![w_0(1), w_0(1)];
        let w = find_work(&mut input, 4);

        assert_eq!(input, vec![]);
        assert_eq!(w.work, vec![]);
        assert_eq!(w.duplicates, vec![w_0(1), w_0(1)]);
        assert_eq!(w.uniques, vec![]);
    }

    #[test]
    fn find_no_work_and_no_duplicates_found() {
        let mut input = vec![w_0(1), w_0(2)];
        let w = find_work(&mut input, 4);

        assert_eq!(input, vec![]);
        assert_eq!(w.work, vec![]);
        assert_eq!(w.duplicates, vec![]);
        assert_eq!(w.duplicates, vec![]);
    }

    #[test]
    fn find_some_work_some_remaining_input_leaves_duplicates() {
        let mut input = vec![w_10(1), w_10(1), w_0(2), w_0(3), w_10(4), w_10(4), w_10(5)];
        let w = find_work(&mut input, 2);

        // Duplicates not seen yet since desired doesn't let us iterate over them
        assert_eq!(input, vec![w_10(1), w_10(1), w_0(2), w_0(3)]);
        assert_eq!(w.work, vec![w_10(4), w_10(4)]);
        assert_eq!(w.duplicates, vec![]);
        assert_eq!(w.uniques, vec![w_10(5)]);
    }

    #[test]
    fn find_work_across_multiple_keys() {
        let mut input = vec![w_10(1), w_10(1), w_10(2), w_10(2)];
        let w = find_work(&mut input, 3);

        // Duplicates not seen yet since desired doesn't let us iterate over them
        assert_eq!(input, vec![]);
        assert_eq!(w.work, vec![w_10(2), w_10(2), w_10(1), w_10(1)]);
        assert_eq!(w.duplicates, vec![]);
        assert_eq!(w.uniques, vec![]);
    }

    #[test]
    fn find_work_across_multiple_keys_other_key_in_the_middle() {
        let mut input = vec![w_10(1), w_10(1), w_10(2), w_10(3), w_10(3)];
        let w = find_work(&mut input, 5);

        // Duplicates not seen yet since desired doesn't let us iterate over them
        assert_eq!(input, vec![]);
        assert_eq!(w.work, vec![w_10(3), w_10(3), w_10(1), w_10(1)]);
        assert_eq!(w.duplicates, vec![]);
        assert_eq!(w.uniques, vec![w_10(2)]);
    }
}
