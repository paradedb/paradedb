import math
import re
import sys

# Constants
missing_result_penalty = 2
missing_result_time = 100  # Example value, replace with actual value
constant_time_add = 10


# Function to calculate the geometric mean of a list of numbers
def geometric_mean(numbers):
    product = 1
    for num in numbers:
        product *= num
    return product ** (1 / len(numbers))


def calculate_summary(filtered_data):
    """
    Score our benchmark results using the ClickBench algorithm:
    The algorithm: for each of the queries,
    - if there is a result - take query duration, add 10 ms, and divide it to the corresponding value from the baseline,
    - if there is no result - take the worse query duration across all query runs for this system and multiply by 2.
    Take geometric mean across the queries.
    """ 
    # Initialize lists to store baseline data and summaries
    baseline_data = []
    summaries = []

    # Convert filtered_data into the expected format (list of lists of lists)
    filtered_data = [[run] for run in filtered_data]

    # Iterate over each query in filtered_data
    for query_num in range(len(filtered_data[0])):
        # Initialize list to store timings for each run of the query
        query_timings = []
        # Iterate over each run for the current query
        for run_num in range(len(filtered_data)):
            # Find the minimum timing for the current run of the query
            min_timing = min(filtered_data[run_num][query_num])
            # Add the minimum timing to the list of timings for the query
            query_timings.append(min_timing)
        # Add the list of timings for the query to the baseline data
        baseline_data.append(query_timings)

    # Calculate the summary for each data point in filtered_data
    for run_timings in filtered_data:
        # Initialize variables to store accumulator and number of used queries
        accumulator = 0
        used_queries = 0
        # Iterate over each query
        for query_num in range(len(run_timings)):
            # Calculate the current timing for the query
            curr_timing = min(run_timings[query_num])
            # Find the corresponding baseline timing
            baseline_timing = min(baseline_data[query_num])
            # Calculate the ratio between current timing and baseline timing
            ratio = (constant_time_add + curr_timing) / (constant_time_add + baseline_timing)
            # Add the logarithm of the ratio to the accumulator
            accumulator += math.log(ratio)
            # Increment the number of used queries
            used_queries += 1
        # Calculate the geometric mean of the accumulated ratios and append to summaries
        summaries.append(math.exp(accumulator / used_queries))

    # Return the summaries
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

    filtered_data = results
    print(results)
    # up to here it seems to be printing properly

    # Example usage
    summaries = calculate_summary(filtered_data)
    print(summaries)

    summary = geometric_mean(summaries)
    print(summary)

    # Calculate the score
    # print("Benchmark Score:", benchmark_score)
