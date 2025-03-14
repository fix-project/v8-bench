file (GLOB LIB_SOURCES "*.cc")

add_library (util STATIC ${LIB_SOURCES})
target_include_directories (util SYSTEM PUBLIC "${PROJECT_SOURCE_DIR}/third-party")
target_include_directories (util INTERFACE .)
