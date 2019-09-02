use super::{Commitment, Error, Hash, Hashable, Index, Merkelizable, Node, Proof, Result};
use std::{collections::VecDeque, ops::Index as IndexOp};

// OPT: Do not store leaf hashes but re-create.
// OPT: Allow up to `n` lower layers to be skipped.
#[derive(Clone, Debug)]
pub struct Tree<'a, Container: Merkelizable> {
    commitment: Commitment,
    nodes:      Vec<Hash>,
    leaves:     &'a Container,
}

impl<'a, Container: Merkelizable> Tree<'a, Container> {
    pub fn from_leaves(leaves: &'a Container) -> Result<Self> {
        let num_leaves = leaves.len();
        if !num_leaves.is_power_of_two() {
            return Err(Error::NumLeavesNotPowerOfTwo);
        }
        let depth = num_leaves.trailing_zeros() as usize;
        let mut nodes = vec![Hash::default(); 2 * num_leaves - 1];

        // Hash the tree
        // TODO: leaves.iter().enumerate()
        for i in 0..leaves.len() {
            // At `depth` there should always be an `i `th leaf.
            let mut cursor = Index::from_depth_offset(depth, i).unwrap();
            nodes[cursor.as_index()] = leaves.leaf_hash(i);
            while cursor.is_right() {
                cursor = cursor.parent().unwrap();
                nodes[cursor.as_index()] = Node(
                    &nodes[cursor.left_child().as_index()],
                    &nodes[cursor.right_child().as_index()],
                )
                .hash()
            }
        }

        Ok(Tree {
            commitment: Commitment::from_depth_hash(depth, &nodes[0])?,
            nodes,
            leaves,
        })
    }

    pub fn commitment(&self) -> &Commitment {
        &self.commitment
    }

    pub fn open(&self, indices: &[usize]) -> Result<Proof> {
        let indices = self.commitment().sort_indices(indices)?;
        let proof_indices: Vec<usize> = indices.iter().map(Index::offset).collect();
        let mut indices: VecDeque<Index> = indices.into_iter().collect();
        let mut hashes: Vec<Hash> = Vec::new();

        while let Some(current) = indices.pop_front() {
            // Root node has no parent and means we are done
            if let Some(parent) = current.parent() {
                // Add parent index to the queue for the next pass
                indices.push_back(parent);

                // Since we have a parent, we must have a sibling
                let sibling = current.sibling().unwrap();

                // Check if we merge with the next merkle index.
                if let Some(&next) = indices.front() {
                    if next == sibling {
                        // Skip next and don't write a decommitment for either
                        let _ = indices.pop_front();
                        continue;
                    }
                }

                // Add a sibling hash to the decommitment
                hashes.push(self[sibling].clone());
            }
        }
        Proof::from_hashes(self.commitment(), &proof_indices, &hashes)
    }
}

impl<Container: Merkelizable> IndexOp<Index> for Tree<'_, Container> {
    type Output = Hash;

    fn index(&self, index: Index) -> &Hash {
        &self.nodes[index.as_index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macros_decl::hex;
    use u256::U256;

    impl Merkelizable for Vec<U256> {
        type Leaf = U256;

        fn len(&self) -> usize {
            self.len()
        }

        fn leaf(&self, index: usize) -> &U256 {
            &self[index]
        }
    }

    #[test]
    fn test_merkle_creation_and_proof() {
        let depth = 6;
        let leaves: Vec<_> = (0..2_u64.pow(depth))
            .map(|i| U256::from((i + 10).pow(3)))
            .collect();

        // Build the tree
        let tree = Tree::from_leaves(&leaves).unwrap();
        let root = tree.commitment();
        assert_eq!(
            root.hash().as_bytes(),
            hex!("fd112f44bc944f33e2567f86eea202350913b11c000000000000000000000000")
        );

        // Open indices
        let indices = vec![1, 11, 14];
        assert_eq!(root.proof_size(&indices).unwrap(), 9);
        let proof = tree.open(&indices).unwrap();
        #[rustfmt::skip]
        assert_eq!(proof.hashes(), &[
            Hash::new(hex!("00000000000000000000000000000000000000000000000000000000000003e8")),
            Hash::new(hex!("0000000000000000000000000000000000000000000000000000000000001f40")),
            Hash::new(hex!("0000000000000000000000000000000000000000000000000000000000003d09")),
            Hash::new(hex!("4ea8b9bafb11dafcfe132a26f8e343eaef0651d9000000000000000000000000")),
            Hash::new(hex!("023a7ce535cadd222093be053ac26f9b800ee476000000000000000000000000")),
            Hash::new(hex!("70b0744af2583d10e7e3236c731d37605e196e06000000000000000000000000")),
            Hash::new(hex!("221aea6e87862ba2d03543d0aa82c6bffee310ae000000000000000000000000")),
            Hash::new(hex!("68b58e5131703684edb16d41b763017dfaa24a35000000000000000000000000")),
            Hash::new(hex!("e108b7dc670810e8588c67c2fde7ec4cc00165e8000000000000000000000000")),
        ]);

        // Verify proof
        let select_leaves: Vec<_> = indices.iter().map(|&i| (i, &leaves[i])).collect();
        proof.verify(&select_leaves).unwrap();

        // Verify non-root
        let non_root = Hash::new(hex!(
            "ed112f44bc944f33e2567f86eea202350913b11c000000000000000000000000"
        ));
        let non_proof = Proof::from_hashes(
            &Commitment::from_depth_hash(root.depth(), &non_root).unwrap(),
            &indices,
            &proof.hashes(),
        )
        .unwrap();
        assert_eq!(
            non_proof.verify(&select_leaves),
            Err(Error::RootHashMismatch)
        );
    }
}
