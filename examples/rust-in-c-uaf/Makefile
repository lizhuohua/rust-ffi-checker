# tool macros
CC := cc
RUSTC := rustc
CCFLAGS := # C compile flags
RUSTCFLAGS := # Rust compile flags
CCOBJFLAGS := $(CCFLAGS) -c

# path macros
BIN_PATH := bin
OBJ_PATH := obj
SRC_PATH := src

# compile macros
TARGET_NAME := rust-in-c-uaf
TARGET := $(BIN_PATH)/$(TARGET_NAME)

# src files & obj files
C_SRC := $(foreach x, $(SRC_PATH), $(wildcard $(addprefix $(x)/*,.c*)))
RUST_SRC := $(foreach x, $(SRC_PATH), $(wildcard $(addprefix $(x)/*,.rs*)))
C_OBJ := $(addprefix $(OBJ_PATH)/, $(addsuffix .o, $(notdir $(basename $(C_SRC)))))
RUST_OBJ := $(addprefix $(OBJ_PATH)/lib, $(addsuffix .a, $(notdir $(basename $(RUST_SRC)))))

# clean files list
CLEAN_LIST := $(TARGET) $(C_OBJ) $(RUST_OBJ)

# default rule
default: makedir all

# non-phony targets
$(TARGET): $(C_OBJ) $(RUST_OBJ)
	$(CC) $(CCFLAGS) -o $@ $(C_OBJ) $(RUST_OBJ) -pthread -ldl

$(OBJ_PATH)/%.o: $(SRC_PATH)/%.c*
	$(CC) $(CCOBJFLAGS) -o $@ $<

$(OBJ_PATH)/lib%.a: $(SRC_PATH)/%.rs*
	$(RUSTC) $(RUSTCFLAGS) -o $@ $<

# phony rules
.PHONY: makedir
makedir:
	@mkdir -p $(BIN_PATH) $(OBJ_PATH)

.PHONY: all
all: $(TARGET)

.PHONY: clean
clean:
	@echo CLEAN $(CLEAN_LIST)
	@rm -f $(CLEAN_LIST)
