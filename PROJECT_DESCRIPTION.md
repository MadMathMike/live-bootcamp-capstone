# Proxy-Based Load Balancer with Adaptive Decision Engine

![](https://t9015495369.p.clickup-attachments.com/t9015495369/42c4df4a-f913-4509-9a1f-3ce027b996aa/Screenshot%202024-06-22%20at%2011.36.41%E2%80%AFPM.png)

# Project Description

Want to use Rust where performance is of utmost importance? Let’s implement a high-performance, adaptive [load balancer](https://en.wikipedia.org/wiki/Load_balancing_(computing)) in Rust!

Rust is known for its safety, speed, and robustness, making it an excellent choice for building fault-tolerant, performance-critical networking and [distributed applications](https://en.wikipedia.org/wiki/Distributed_computing).

This custom proxy-based load balancer will intelligently distribute incoming network traffic across multiple backend worker servers, based on various real-time performance data. This project will involve implementing load balancing algorithms, health monitoring, and performance optimization.

Can’t wait to start? Let’s jump right in!

# Project Objectives

1. **Understand load balancing fundamentals**
    *   Learn the basics of load balancing, including traffic distribution, health monitoring, and performance optimization.

1. **Implement a load balancer in Rust**
    *   Gain hands-on experience with Rust by building a load balancer from scratch, and incrementally optimize for performance.

1. **Solidify your Rust skills**
    *   Apply the Rust knowledge and best practices acquired throughout this program to build a robust and efficient load balancer.

# Project Requirements

1. **Load balancer structure**
    *   **Receive incoming requests:** Use a popular framework like Axum or Hyper.
    *   **Define a worker servers list:** Maintain a list of backend worker servers and corresponding addresses.
    *   **Reroute incoming requests:** Distribute incoming requests to worker servers using different load balancing algorithms (e.g., round-robin, least connections).

1. **Health checks**
    *   **Implement health check endpoints:** Ensure worker servers have a health check endpoint to report their status.
    *   **Check health status using the load balancer:** Periodically check the health of worker servers.

1. **Load balancing algorithms**
    *   **Round-robin:** Distribute requests evenly across all worker servers.
    *   **Least connections:** Route requests to the worker server with the least active connections.

1. **Adaptive load balancing with decision engine**
    *   **Real-time data:** Continuously collect performance data such as response times and concurrent connections.
    *   **Decision engine:** Analyze the accumulated data to make decisions about which load balancing algorithm to use.
    *   **Adaptive algorithm switching:** Dynamically switch between different load balancing algorithms based on the decisions made by the decision engine.

# Project Milestones

### Week 1
1. **Understand load balancing fundamentals**
    *   Study fundamentals of distributed systems and load balancing, such as:
        *   DNS-based vs proxy-based/application-layer load balancing
        *   Software vs hardware balancers
        *   Common load balancing algorithms (e.g., round-robin, least connections).
    *   Go through the following recommended resources:
        *   [What is a distributed system?](https://youtu.be/ajjOEltiZm4)
        *   [Why we need a load balancer](https://youtu.be/sCR3SAVdyCc)
        *   [Most common load balancing algorithms](https://youtu.be/dBmxNsS3BGE)
        *   [Create an HTTP proxy server in Rust with hyper](https://www.youtube.com/watch?v=cICaUDqZ5t0)
            *   This is very close to a load balancer! The additional work involves forwarding requests to a list of worker servers!
2. **Brainstorm**
    *   Plan how to apply the Rust concepts and best practices you've learned in the program to this project.
3. **Experiment**
    *   Create small Rust programs to get familiar with load balancing algorithms like:
        *   Round-robin
        *   Least connections
4. **Notify your instructor**
    *   Inform your instructor on Discord that you have chosen this project.

### Week 2
1. **Create a worker server**
    *   Add a health check endpoint
        *   Simply return `200 OK` for now, indicating a healthy server.
    *   Add a work endpoint
        *   Return `200 OK` after a 10ms delay that simulates work being done.
    *   Start up multiple worker server instances
2. **Create a basic load balancer server**
    *   Round-robin algorithm
        *   Implement a simple round-robin algorithm to distribute requests evenly.
        *   Test with worker servers to ensure proper request distribution.
        *   Use a client like Postman to send traffic to the load balancer.
    *   Additional load balancing algorithms
        *   Implement and test at least one other algorithm, such as least connections.
    *   Manual run-time algorithm switching
        *   Implement the ability to switch algorithms at runtime via an endpoint on the load balancer.
3. **Test under various conditions to see how each algorithm performs**
    *   Use a client like Postman to send traffic to the load balancer
    *   Update the work endpoint to artificially introduce conditions like prolonged request processing and elevated error rates.
    *   Evaluate the performance impact of different load balancing algorithms under various conditions.
4. **Showcase progress**
    *   Notify your instructor if you’d like to showcase your work on Discord or during a live call.
    *   Share your progress on Discord and/or during a live call.

### Week 3
1. **Implement adaptive load balancing**
    *   Real-time data and conditions
        *   Define conditions under which algorithm switches should occur.
        *   Start with straightforward conditions that are easy to (re)produce, such as prolonged request processing time or elevated error rates.
        *   For now, collect these metrics about worker servers within the load balancer using simple in-memory data structures (e.g., HashMap). No need to use third-party services like Datadog.
        *   Prevent overhead and maintain system stability by limiting algorithm switches (e.g., at most one switch per 60-second window).
        *   For now, make sure these conditions can be artificially triggered by updating the worker servers.
2. **Implement decision engine**
    *       *   Develop a decision engine to monitor and evaluate the defined conditions.
        *   Implement functionality to signal when to switch algorithms based on triggered conditions.
3. **Adaptive load balancing**
    *       *   Transition to using the decision engine for runtime algorithm switching, replacing the manual approach.
        *   Monitor the system to ensure the load balancer responds correctly to changing conditions and optimizes performance.
4. **Showcase Progress**
    *       *   Notify your instructor if you’d like to showcase your work on Discord or during a live call.
        *   Share your progress on Discord and/or during a live call.

### Week 4
1. **Refactoring**
    *   Make sure to incorporate the best patterns and practices we've learned during this program into your projects.
2. **Bug fixes**
    *   Fix any remaining bugs.
3. **Optional enhancements**
    *   Implement any additional features and put the last finishing touches on your project. See the list of optional enhancements below!
4. **Demo prep**
    *   Prepare to demo your work at the final capstone demo day!

# Optional Enhancements

1. **External observability service**
    *   Capture more extensive metrics using an external observability service like Datadog.
    *   Enhance the decision-making of the decision engine by monitoring more sophisticated metrics captured by this external service.

1. **Fault tolerance**
    *   Ensure the load balancer continues to operate correctly during partial or complete failures.
    *   Implement redundancy and failover mechanisms to handle load balancer failures gracefully.

1. **Deployment**
    *   Containerize the load balancer and backend servers with Docker for consistent deployment.
    *   Integrate with a container orchestrator like Kubernetes to manage the load balancer and backend services as needed.

_(Note that these enhancements are optional and should only be attempted after the core load balancer functionality is working.)_

# Template and Examples

**Starter template:** [https://github.com/letsgetrusty/load-balancer](https://github.com/letsgetrusty/load-balancer)

**Example implementations from previous students:**
*   [https://github.com/letsgetrusty/load-balancer-ex-1](https://github.com/letsgetrusty/load-balancer-ex-1)
*   [https://github.com/letsgetrusty/load-balancer-ex-2](https://github.com/letsgetrusty/load-balancer-ex-2)
*   [https://github.com/letsgetrusty/load-balancer-ex-3](https://github.com/letsgetrusty/load-balancer-ex-3)
*   [https://github.com/letsgetrusty/load-balancer-ex-4](https://github.com/letsgetrusty/load-balancer-ex-4)
*   [https://github.com/letsgetrusty/load-balancer-ex-5](https://github.com/letsgetrusty/load-balancer-ex-5)