.PHONY: build compile build-lit test test-all sexpr

CMAKE_INSTALL_PREFIX ?= $(abspath ../../build/install)
MIM_PLUGIN_PATH ?= $(abspath lit)
MIM_BINARY_PATH ?= $(abspath ../../build/install/bin/mim.exe)
MIM_DEBUG_BIN_PATH ?= $(abspath ../../build/bin/Debug/mim.exe)

# use flag "-G Ninja" to generate compile_commands.json on Windows
build:
	cmake -S ../../ -B ../../build \
		-DBUILD_TESTING=ON \
		-DMIM_BUILD_EXAMPLES=ON \
		-DCMAKE_EXPORT_COMPILE_COMMANDS=1 \
		-DCMAKE_INSTALL_PREFIX=$(CMAKE_INSTALL_PREFIX)


compile:
	cmake --build ../../build -j 8 --target install -- /verbosity:quiet

build-lit:
	cmake --build ../../build -j 8 --target lit

test:
	MIM_PLUGIN_PATH=$(MIM_PLUGIN_PATH) && $(MIM_BINARY_PATH) ./lit/$(TEST) -o -

test-all:
	python ../../lit/lit ../../build/lit -v --filter eqsat

sexpr:
	mim ./lit/$(TEST) --sexpr-include-types --output-sexpr-slotted -

gdb:
	gdb $(MIM_BINARY_PATH)
