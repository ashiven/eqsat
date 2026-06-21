#pragma once

#include <iostream>

/****************** DEBUG *********************/
inline constexpr bool DEBUG       = true;
inline constexpr bool SCOPES      = false;
inline constexpr bool PERFORMANCE = false;

template<bool DBG_KIND = DEBUG, typename... Args>
void dbg(Args&&... args) {
    if constexpr (DBG_KIND) (std::cout << ... << std::forward<Args>(args)) << "\n";
}

template<bool DBG_KIND = DEBUG, typename... Args>
void dbg_(Args&&... args) {
    if constexpr (DBG_KIND) (std::cout << ... << std::forward<Args>(args));
}

#define START_TIMER(name) auto _start_##name = std::chrono::steady_clock::now();
#define END_TIMER(name)                                                                                             \
    {                                                                                                               \
        auto _end_##name = std::chrono::steady_clock::now();                                                        \
        if constexpr (PERFORMANCE) {                                                                                \
            std::cout << #name << " took: "                                                                         \
                      << std::chrono::duration_cast<std::chrono::milliseconds>(_end_##name - _start_##name).count() \
                      << "ms\n";                                                                                    \
        }                                                                                                           \
    }
