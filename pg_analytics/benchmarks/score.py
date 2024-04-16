import math
import re
import sys

def score_benchmark(query_times):
    """
    Score our benchmark results using the ClickBench algorithm:
    The algorithm: for each of the queries,
    - if there is a result - take query duration, add 10 ms, and divide it to the corresponding value from the baseline,
    - if there is no result - take the worse query duration across all query runs for this system and multiply by 2.
    Take geometric mean across the queries.
    """    
    summaries = []
    for times in query_times:
        non_null_times = [time for time in times if time is not None]
        if non_null_times:
            geometric_mean = math.exp(sum(math.log(time) for time in non_null_times) / len(non_null_times))
            summaries.append(geometric_mean)
        else:
            summaries.append(None)
    
    return summaries

if __name__ == "__main__":
    # Read input from stdin
    input_results = sys.stdin.readlines()

    # Parse the input into a list of lists of floats
    results = []
    for row in input_results:
        # Use regular expression to extract numeric values
        values = re.findall(r'\d+\.\d+', row)
        # Convert each value to float
        results.append([float(value) for value in values])

    print(results)

    # Calculate the score
    benchmark_score = score_benchmark(results)
    print("Benchmark Score:", benchmark_score)
