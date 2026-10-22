//! `RawNet`: the testkit's own plain net representation, independent of petrivet's types.
//!
//! Ordinary arcs only. Place `i` becomes PNML id `p{i}`, transition `j` becomes `t{j}`
//! (petrivet's exporter numbers nodes in the order the builder created them, which is the
//! order used here; a test pins this down).

use petrivet::net::{Net, Place};
use petrivet::system::PetriNet;
use petrivet::system::marking::Marking;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawNet {
    pub places: usize,
    /// Per transition: sorted, duplicate-free input places.
    pub pre: Vec<Vec<usize>>,
    /// Per transition: sorted, duplicate-free output places.
    pub post: Vec<Vec<usize>>,
    pub marking: Vec<u32>,
}

impl RawNet {
    /// Builds from arc lists `(place, transition)` and `(transition, place)`; duplicates collapse.
    pub fn from_arcs(places: usize, transitions: usize, pt: &[(usize, usize)], tp: &[(usize, usize)]) -> Self {
        let mut pre = vec![Vec::new(); transitions];
        let mut post = vec![Vec::new(); transitions];
        for &(p, t) in pt {
            pre[t].push(p);
        }
        for &(t, p) in tp {
            post[t].push(p);
        }
        for v in pre.iter_mut().chain(post.iter_mut()) {
            v.sort_unstable();
            v.dedup();
        }
        RawNet { places, pre, post, marking: vec![0; places] }
    }

    pub fn transitions(&self) -> usize {
        self.pre.len()
    }

    pub fn arcs(&self) -> usize {
        self.pre.iter().chain(&self.post).map(Vec::len).sum()
    }

    pub fn tokens_total(&self) -> u32 {
        self.marking.iter().sum()
    }

    /// Weak connectivity (union-find), the property petrivet requires of every net.
    pub fn is_connected(&self) -> bool {
        let n = self.places + self.transitions();
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }
        for t in 0..self.transitions() {
            for &p in self.pre[t].iter().chain(&self.post[t]) {
                let (a, b) = (find(&mut parent, p), find(&mut parent, self.places + t));
                parent[a] = b;
            }
        }
        let root = find(&mut parent, 0);
        (0..n).all(|x| find(&mut parent, x) == root)
    }

    pub fn is_enabled(&self, m: &[u32], t: usize) -> bool {
        self.pre[t].iter().all(|&p| m[p] >= 1)
    }

    /// Fires `t` in place; returns false (and leaves `m` untouched) if it is not enabled.
    pub fn fire(&self, m: &mut [u32], t: usize) -> bool {
        if !self.is_enabled(m, t) {
            return false;
        }
        self.pre[t].iter().for_each(|&p| m[p] -= 1);
        self.post[t].iter().for_each(|&p| m[p] += 1);
        true
    }

    /// Replays a firing sequence from the initial marking; `None` if some step is not enabled.
    pub fn replay(&self, seq: &[usize]) -> Option<Vec<u32>> {
        let mut m = self.marking.clone();
        seq.iter().all(|&t| self.fire(&mut m, t)).then_some(m)
    }

    /// Builds the petrivet system. Err if petrivet rejects the net (degenerate or disconnected).
    pub fn to_petrivet(&self) -> Result<PetriNet<Net>, petrivet::net::builder::NetError> {
        let mut b = Net::builder();
        let ps: Vec<Place> = (0..self.places).map(|_| b.add_place()).collect();
        for t in 0..self.transitions() {
            let tr = b.add_transition();
            self.pre[t].iter().for_each(|&p| { b.add_arc((ps[p], tr)); });
            self.post[t].iter().for_each(|&p| { b.add_arc((tr, ps[p])); });
        }
        let net = b.build()?;
        let marking: Marking<u32> = ps.iter().zip(&self.marking).map(|(&p, &k)| (p, k)).collect();
        Ok(PetriNet::new(net, marking))
    }

    /// PNML text via petrivet's exporter.
    pub fn to_pnml(&self) -> Result<String, String> {
        let sys = self.to_petrivet().map_err(|e| e.to_string())?;
        sys.to_pnml().map_err(|e| e.to_string())
    }
}
