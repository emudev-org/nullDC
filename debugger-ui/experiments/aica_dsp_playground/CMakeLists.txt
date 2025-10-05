cmake_minimum_required(VERSION 3.16)

project(aica-dsp)

enable_language(CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED On)

list(APPEND sources
    dsp/dsp_interp.cpp
    dsp/dsp.cpp
    dsp/aica.cpp
)

if (EMSCRIPTEN)
    set(CMAKE_EXECUTABLE_SUFFIX ".wasm")
else()
    list(APPEND sources
        ui/sdl.cpp
    )
endif()

add_executable(aica-dsp ${sources})

if (EMSCRIPTEN)
    set_target_properties(aica-dsp PROPERTIES COMPILE_FLAGS "-O3 -fwrapv")
    set_target_properties(aica-dsp PROPERTIES LINK_FLAGS    "-O3 -s WASM=1 -s STANDALONE_WASM --no-entry -s ASSERTIONS=1")
else()
    find_package(SDL2 REQUIRED)
    include_directories(${SDL2_INCLUDE_DIRS})
    target_link_libraries(aica-dsp ${SDL2_LIBRARIES})
endif()