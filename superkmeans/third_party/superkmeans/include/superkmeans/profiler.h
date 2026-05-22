#pragma once

#include <algorithm>
#include <chrono>
#include <iomanip>
#include <iostream>
#include <mutex>
#include <string>
#include <unordered_map>
#include <vector>

// Disclamer: Code produced by Opus 4.5

namespace skmeans {

/**
 * @brief A centralized profiler for timing code sections.
 *
 * Usage:
 *   // Start/stop manually:
 *   Profiler::Get().Start("my_section");
 *   // ... code ...
 *   Profiler::Get().Stop("my_section");
 *
 *   // Or use RAII scoped timer:
 *   {
 *       SKM_PROFILE_SCOPE("my_section");
 *       // ... code automatically timed ...
 *   }
 *
 *   // Print results:
 *   Profiler::Get().Print();
 *
 *   // Reset all timers:
 *   Profiler::Get().Reset();
 */
class Profiler {
  public:
    struct TimerData {
        size_t accum_time_ns = 0; // Accumulated time in nanoseconds
        size_t call_count = 0;
        std::chrono::high_resolution_clock::time_point start;
        bool running = false;
    };

    // Get the global profiler instance
    static Profiler& Get() {
        static Profiler instance;
        return instance;
    }

    // Start timing a named section
    void Start(const std::string& name) {
        std::lock_guard<std::mutex> lock(mutex_);
        auto& timer = timers_[name];
        if (!timer.running) {
            timer.start = std::chrono::high_resolution_clock::now();
            timer.running = true;
        }
    }

    // Stop timing a named section
    void Stop(const std::string& name) {
        auto end = std::chrono::high_resolution_clock::now();
        std::lock_guard<std::mutex> lock(mutex_);
        auto it = timers_.find(name);
        if (it != timers_.end() && it->second.running) {
            it->second.accum_time_ns +=
                std::chrono::duration_cast<std::chrono::nanoseconds>(end - it->second.start)
                    .count();
            it->second.call_count++;
            it->second.running = false;
        }
    }

    // Get accumulated time in seconds for a timer
    double GetTimeSeconds(const std::string& name) const {
        std::lock_guard<std::mutex> lock(mutex_);
        auto it = timers_.find(name);
        if (it != timers_.end()) {
            return it->second.accum_time_ns / 1e9;
        }
        return 0.0;
    }

    // Get accumulated time in nanoseconds for a timer
    size_t GetTimeNanos(const std::string& name) const {
        std::lock_guard<std::mutex> lock(mutex_);
        auto it = timers_.find(name);
        if (it != timers_.end()) {
            return it->second.accum_time_ns;
        }
        return 0;
    }

    // Get call count for a timer
    size_t GetCallCount(const std::string& name) const {
        std::lock_guard<std::mutex> lock(mutex_);
        auto it = timers_.find(name);
        if (it != timers_.end()) {
            return it->second.call_count;
        }
        return 0;
    }

    // Reset a specific timer
    void Reset(const std::string& name) {
        std::lock_guard<std::mutex> lock(mutex_);
        auto it = timers_.find(name);
        if (it != timers_.end()) {
            it->second.accum_time_ns = 0;
            it->second.call_count = 0;
            it->second.running = false;
        }
    }

    // Reset all timers
    void Reset() {
        std::lock_guard<std::mutex> lock(mutex_);
        timers_.clear();
    }

    // Print all timers with formatting
    void Print(std::ostream& os = std::cout) const {
        std::lock_guard<std::mutex> lock(mutex_);

        // Calculate total time for percentage calculation
        size_t total_ns = 0;
        for (const auto& [name, data] : timers_) {
            total_ns += data.accum_time_ns;
        }

        // Collect and sort timer names for consistent output
        std::vector<std::string> names;
        names.reserve(timers_.size());
        for (const auto& [name, _] : timers_) {
            names.push_back(name);
        }
        std::sort(names.begin(), names.end());

        os << std::fixed << std::setprecision(3);
        os << "\n========== PROFILER RESULTS ==========\n";

        for (const auto& name : names) {
            const auto& data = timers_.at(name);
            double secs = data.accum_time_ns / 1e9;
            double pct = total_ns > 0 ? (data.accum_time_ns * 100.0 / total_ns) : 0.0;

            os << std::left << std::setw(35) << name << std::right << std::setw(10) << secs << "s"
               << " (" << std::setw(5) << pct << "%)"
               << " [" << data.call_count << " calls]"
               << "\n";
        }

        os << "---------------------------------------\n";
        os << std::left << std::setw(35) << "TOTAL" << std::right << std::setw(10)
           << (static_cast<double>(total_ns) / 1e9) << "s\n";
        os << "=======================================\n";
    }

    // Print a hierarchical view (timers with '/' are grouped)
    void PrintHierarchical(std::ostream& os = std::cout) const {
        std::lock_guard<std::mutex> lock(mutex_);

        // Calculate total time from top-level timers only
        size_t total_ns = 0;
        for (const auto& [name, data] : timers_) {
            if (name.find('/') == std::string::npos) {
                total_ns += data.accum_time_ns;
            }
        }

        os << std::fixed << std::setprecision(3);
        os << "\n========== PROFILER RESULTS ==========\n";

        // Group timers by prefix
        std::unordered_map<std::string, std::vector<std::string>> groups;
        std::vector<std::string> top_level;

        for (const auto& [name, _] : timers_) {
            auto pos = name.find('/');
            if (pos != std::string::npos) {
                std::string parent = name.substr(0, pos);
                groups[parent].push_back(name);
            } else {
                top_level.push_back(name);
            }
        }

        // Sort top-level by accumulated time (descending)
        std::sort(
            top_level.begin(),
            top_level.end(),
            [this](const std::string& a, const std::string& b) {
                return timers_.at(a).accum_time_ns > timers_.at(b).accum_time_ns;
            }
        );

        for (const auto& name : top_level) {
            const auto& data = timers_.at(name);
            double secs = data.accum_time_ns / 1e9;
            double pct = total_ns > 0 ? (data.accum_time_ns * 100.0 / total_ns) : 0.0;

            os << std::left << std::setw(40) << name << std::right << std::setw(10) << secs << "s"
               << " (" << std::setw(5) << pct << "%)";
            if (data.call_count > 1) {
                os << " [" << data.call_count << " calls]";
            }
            os << "\n";

            // Print children (sorted by time descending)
            auto it = groups.find(name);
            if (it != groups.end()) {
                auto& children = it->second;
                std::sort(
                    children.begin(),
                    children.end(),
                    [this](const std::string& a, const std::string& b) {
                        return timers_.at(a).accum_time_ns > timers_.at(b).accum_time_ns;
                    }
                );
                for (const auto& child : children) {
                    const auto& child_data = timers_.at(child);
                    double child_secs = child_data.accum_time_ns / 1e9;
                    double child_pct =
                        total_ns > 0 ? (child_data.accum_time_ns * 100.0 / total_ns) : 0.0;
                    std::string short_name = "  - " + child.substr(name.length() + 1);

                    os << std::left << std::setw(40) << short_name << std::right << std::setw(10)
                       << child_secs << "s"
                       << " (" << std::setw(5) << child_pct << "%)";
                    if (child_data.call_count > 1) {
                        os << " [" << child_data.call_count << " calls]";
                    }
                    os << "\n";
                }
            }
        }

        os << "-------------------------------------------\n";
        os << std::left << std::setw(40) << "TOTAL" << std::right << std::setw(10)
           << (static_cast<double>(total_ns) / 1e9) << "s\n";
        os << "===========================================\n";
    }

    // Check if profiling is enabled
    bool IsEnabled() const { return enabled_; }

    // Enable/disable profiling globally
    void SetEnabled(bool enabled) { enabled_ = enabled; }

  private:
    Profiler() = default;
    ~Profiler() = default;
    Profiler(const Profiler&) = delete;
    Profiler& operator=(const Profiler&) = delete;

    mutable std::mutex mutex_;
    std::unordered_map<std::string, TimerData> timers_;
    bool enabled_ = true;
};

/**
 * @brief RAII scoped timer that automatically starts on construction and stops on destruction.
 */
class ScopedTimer {
  public:
    explicit ScopedTimer(std::string name) : name_(std::move(name)) {
        if (Profiler::Get().IsEnabled()) {
            Profiler::Get().Start(name_);
        }
    }

    ~ScopedTimer() {
        if (Profiler::Get().IsEnabled()) {
            Profiler::Get().Stop(name_);
        }
    }

    // Non-copyable, non-movable
    ScopedTimer(const ScopedTimer&) = delete;
    ScopedTimer& operator=(const ScopedTimer&) = delete;
    ScopedTimer(ScopedTimer&&) = delete;
    ScopedTimer& operator=(ScopedTimer&&) = delete;

  private:
    std::string name_;
};

// Convenience macros for profiling
// Helper macros for unique variable name generation
#define SKM_CONCAT_IMPL(x, y) x##y
#define SKM_CONCAT(x, y) SKM_CONCAT_IMPL(x, y)

// Profiling macros - only enabled when BENCHMARK_TIME is defined
#ifdef BENCHMARK_TIME
// SKM_PROFILE_SCOPE creates a scoped timer with the given name
#define SKM_PROFILE_SCOPE(name) ::skmeans::ScopedTimer SKM_CONCAT(_skm_timer_, __LINE__)(name)

// SKM_PROFILE_FUNCTION creates a scoped timer with the function name
#define SKM_PROFILE_FUNCTION() SKM_PROFILE_SCOPE(__func__)

// Manual start/stop macros
#define SKM_PROFILE_START(name) ::skmeans::Profiler::Get().Start(name)
#define SKM_PROFILE_STOP(name) ::skmeans::Profiler::Get().Stop(name)
#else
// No-op macros when profiling is disabled
#define SKM_PROFILE_SCOPE(name) ((void) 0)
#define SKM_PROFILE_FUNCTION() ((void) 0)
#define SKM_PROFILE_START(name) ((void) 0)
#define SKM_PROFILE_STOP(name) ((void) 0)
#endif

} // namespace skmeans
