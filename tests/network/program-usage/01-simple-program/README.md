# Simple Program Example

This example demonstrates the basics of WASM programs in Modality contracts.

## What This Example Does

Creates a simple program that:
- Takes input arguments
- Performs a computation
- Produces commit actions (POST actions)

## Steps

### 1. Create the Program

```bash
./01-create-program.sh
```

Creates a new program project using `modal program create`.

### 2. Build the Program

```bash
./02-build-program.sh
```

Compiles the program to WASM.

### 3. Create a Contract and Upload Program

```bash
./03-upload-program.sh
```

Creates a test contract and uploads the program to `/__programs__/simple_program.wasm`.

### 4. Invoke the Program

```bash
./04-invoke-program.sh
```

Creates a commit with an "invoke" action that executes the program. The program produces POST actions that are automatically processed.

## Program Logic

The simple program:
1. Accepts `message` and `count` arguments
2. Posts the message to `/data/message`
3. Posts the count to `/data/count`
4. Posts a timestamp to `/data/executed_at`

## Key Concepts

- **Program vs Predicate**: Programs produce actions, predicates evaluate to true/false
- **Storage Path**: Programs are stored at `/__programs__/{name}.wasm`
- **Invocation**: Users sign the invoke action, validators execute the program
- **Security**: User signature on invoke = indirect signature on results

## Expected Output

After invocation, the contract will have:
- `/data/message` = "Hello from program"
- `/data/count` = 42
- `/data/executed_at` = [timestamp]

