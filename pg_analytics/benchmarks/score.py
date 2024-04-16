# -*- coding: utf-8 -*-
import math
import re
import sys

MISSING_RESULT_PENALTY_FACTOR = 2
MISSING_RESULT_TIME = 100  # This is a no-op since we don't have missing results
CONSTANT_TIME_ADD = 0.010  # 10 ms in seconds


def geometric_mean(numbers):
    """
    Calculate the geometric mean of a list of floats.
    """
    product = 1
    for num in numbers:
        product *= num
    return product ** (1 / len(numbers))


def calculate_summary(filtered_data):
    """
    Score our benchmark results using the ClickBench algorithm:
    The algorithm: for each of the queries,
    - if there is a result - take query duration, add 10 ms, and divide it to
      the corresponding value from the baseline,
    - if there is no result - take the worse query duration across all query
      runs for this system and multiply by 2.
    Take geometric mean across the queries.
    """
    baseline_data = []
    summaries = []

    # Convert filtered_data into the expected format (list of lists of lists)
    filtered_data = [[run] for run in filtered_data]

    # Iterate over each query in filtered_data
    for query_num, _ in enumerate(filtered_data[0]):
        query_timings = []
        # Iterate over each run for the current query
        for run_num, _ in enumerate(filtered_data):
            # Find the minimum timing for the current run of the query
            min_timing = min(filtered_data[run_num][query_num])
            # Add the minimum timing to the list of timings for the query
            query_timings.append(min_timing)
        # Add the list of timings for the query to the baseline data
        baseline_data.append(query_timings)

    # Calculate the summary for each data point in filtered_data
    for run_timings in filtered_data:
        accumulator = 0
        used_queries = 0
        # Iterate over each query
        for query_num, query_timing in enumerate(run_timings):
            # Calculate the current timing for the query
            curr_timing = min(query_timing)
            # Find the corresponding baseline timing
            baseline_timing = min(baseline_data[query_num])
            # Calculate the ratio between current timing and baseline timing
            ratio = (CONSTANT_TIME_ADD + curr_timing) / (
                CONSTANT_TIME_ADD + baseline_timing
            )
            # Add the logarithm of the ratio to the accumulator
            accumulator += math.log(ratio)
            # Increment the number of used queries
            used_queries += 1
        # Calculate the geometric mean of the accumulated ratios and append to summaries
        summaries.append(math.exp(accumulator / used_queries))

    return summaries


if __name__ == "__main__":
    input_results = sys.stdin.readlines()

    # Parse the input into a list of lists of floats
    results = []
    for row in input_results:
        values = re.findall(r"\d+\.\d+", row)
        results.append([float(value) for value in values])

    score_summaries = calculate_summary(results)
    final_score = geometric_mean(score_summaries)
    print("Benchmark Score:", final_score)
