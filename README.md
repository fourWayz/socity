# Socity

## Overview
This is a decentralized task management built on the Internet Computer Protocol (ICP). It allows users to create, update, complete, and delete tasks. Users can view all tasks, view completed tasks, and retrieve individual tasks by their unique identifiers. The system provides authorization checks to ensure that only the task owner can modify or delete their tasks

## Installation

### Starts the replica, running in the background
```bash dfx start --background ```

### Deploys your canisters to the replica and generates your candid interface
```bash dfx deploy ```

## Usage

### Adding a Task

To add a task, you can use the following command:

```bash
dfx canister call icp_rust_boilerplate_backend add_task '(
  record {
    title = "Task Title";
    description = "Task Description";
    deadline = null; # Optional deadline in UNIX timestamp format
  }
)'
```

### Getting a Task

```bash
dfx canister call icp_rust_boilerplate_backend get_task '(0)'
```

### Updating a task

```bash
dfx canister call icp_rust_boilerplate_backend update_task '(0, record { title = "New Title"; description = "New Description"; deadline = null; })'
```

### Completing a task

```bash
dfx canister call icp_rust_boilerplate_backend complete_task '(0)'
```

### Getting completed tasks

```bash
dfx canister call icp_rust_boilerplate_backend get_completed_tasks
```

### Getting all tasks

```bash
dfx canister call icp_rust_boilerplate_backend get_all_tasks
```

### Deleting a task

```bash
dfx canister call icp_rust_boilerplate_backend delete_task '(0)'
```