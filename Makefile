
test:
	nvim --headless \
		--noplugin \
		-u scripts/minimal_init.vim \
		-c "PlenaryBustedDirectory tests/functional/ {minimal_init = 'tests/minimal_init.vim'}"
