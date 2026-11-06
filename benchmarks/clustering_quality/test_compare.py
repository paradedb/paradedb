import tempfile
import unittest
from pathlib import Path

import numpy as np
from compare import evaluate, exact_topk, unit_rows


class StoredMembershipEvaluation(unittest.TestCase):
    def test_does_not_replace_stored_memberships_with_nearest_centroids(self):
        vectors = unit_rows([[1, 0], [0.9, 0.1], [-1, 0], [-0.9, 0.1]])
        centroids = np.array([[1, 0], [-1, 0]], dtype=np.float32)
        labels = np.array([1, 1, 0, 0])
        truth = np.array([[0], [2]])
        summary, recall, work = evaluate(
            vectors, centroids, truth, centroids, labels, np.array([2, 2])
        )
        self.assertEqual(summary["non_nearest_membership_fraction"], 1)
        self.assertGreater(summary["mean_cosine_loss"], 1.9)
        np.testing.assert_array_equal(recall, [0, 0, 1])
        np.testing.assert_array_equal(work, [0, 2, 4])

    def test_streaming_ground_truth_matches_dense_with_ties(self):
        rng = np.random.default_rng(21)
        vectors = rng.normal(size=(37, 8)).astype(np.float32)
        vectors[8:14] = vectors[0]
        queries = unit_rows(np.vstack([vectors[0], rng.normal(size=(3, 8))]))
        expected = np.argsort(-(queries @ unit_rows(vectors).T), axis=1, kind="stable")[
            :, :10
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "vectors.npy"
            np.save(path, vectors)
            mapped = np.load(path, mmap_mode="r")
            actual = exact_topk(mapped, queries, batch_size=5)
        np.testing.assert_array_equal(actual, expected)

    def test_sampled_nearest_diagnostic_still_evaluates_every_stored_assignment(self):
        vectors = unit_rows([[1, 0], [0.9, 0.1], [-1, 0], [-0.9, 0.1]])
        centroids = np.array([[1, 0], [-1, 0]], dtype=np.float32)
        labels = np.array([1, 1, 0, 0])
        summary, recall, _ = evaluate(
            vectors,
            centroids,
            np.array([[0], [2]]),
            centroids,
            labels,
            np.array([2, 2]),
            2,
        )
        self.assertEqual(summary["loss_evaluation_rows"], 4)
        self.assertEqual(summary["nearest_membership_diagnostic_rows"], 2)
        self.assertGreater(summary["mean_cosine_loss"], 1.9)
        np.testing.assert_array_equal(recall, [0, 0, 1])

    def test_replicas_improve_coverage_but_count_as_extra_posting_work(self):
        vectors = np.array([[1, 0], [-1, 0]], dtype=np.float32)
        labels = np.array([0, 1, 1])
        rows = np.array([0, 0, 1])
        summary, recall, work = evaluate(
            vectors,
            vectors,
            np.array([[0], [1]]),
            vectors,
            labels,
            np.array([1, 2]),
            row_ids=rows,
        )
        self.assertEqual(summary["replication_factor"], 1.5)
        self.assertEqual(summary["mean_cosine_loss"], 0)
        self.assertAlmostEqual(summary["mean_posting_cosine_loss"], 2 / 3)
        np.testing.assert_array_equal(recall, [0, 1, 1])
        np.testing.assert_array_equal(work, [0, 1.5, 3])


if __name__ == "__main__":
    unittest.main()
