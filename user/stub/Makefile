
build:
	gcc main.s				\
		-ffreestanding 		\
		-nostdlib			\
		-static				\
		-masm=intel			\
		-Wl,-nostdlib		\
		-nostartfiles		\
		-o init.elf

build-release:
	gcc main.s 				\
		-ffreestanding 		\
		-nostdlib			\
		-static				\
		-masm=intel			\
		-Wl,-nostdlib		\
		-nostartfiles		\
		-o init.elf

install:
	cp -v init.elf $(INSTALL_DIR)

fmt:

clean:
	rm -f init.elf

