# A Capability-Oriented Application Platform

*Project Proposal and Architecture*
## 1. Project Overview

### 1.1 What Are We Building?

We are building a modular, capability-oriented application platform: a reusable foundation for creating, executing, connecting, and managing independent applications while maintaining explicit control over what each application is allowed to access.

The platform is intended to make independently developed software easier to compose into larger systems. Applications should be able to expose useful functionality through typed interfaces, consume capabilities provided by other applications or services, and operate without sharing unrestricted access to one another's implementation or resources.

A central design principle is:

> Applications should be isolated by default, but able to communicate and share resources through explicitly granted, typed capabilities.

The platform is particularly relevant in an environment where applications are frequently created or modified by AI agents. As generating software becomes less expensive, integrating, operating, securing, and reusing that software becomes a more significant challenge. The platform aims to make those activities coherent parts of the application model rather than a collection of ad hoc deployment and integration tasks.

The project is not simply a TypeScript runtime, a desktop application manager, or a general-purpose orchestration system. It is a reusable application execution and composition foundation that different host products and environments can use.

### 1.2 Runtime, Platform, and Hosts

The project has three related but distinct parts:

**Application Runtime**

The runtime executes an individual application within a controlled environment. It provides the language execution environment, loads application code, and exposes only the capabilities granted by the host. The runtime is a foundational component, not the complete product.

**Application Platform**

The platform builds on the runtime to define and manage applications and their relationships. It provides the application model, identity, manifests, capability requirements and grants, provider connections, application interfaces, and lifecycle abstractions. The platform is the primary product of this project.

**Host Applications**

A host is a product or system that uses the platform to provide a particular user experience and operating environment. A host may supply configuration, policy, presentation, orchestration, and integrations while relying on the platform for application execution and composition.

These boundaries should be reflected in programmatic interfaces. The platform should not depend on a particular desktop shell, server interface, or orchestration product. A host should be replaceable without requiring application logic or the runtime to be rewritten.

### 1.3 The Intended Application Model

An application is a logical unit with its own identity, dependencies, requirements, configuration, persistent state, lifecycle, and interfaces. It should not need to know whether its capabilities are implemented locally, provided by another application, or hosted on a remote machine.

An application may expose functionality to other applications, provide one or more user interfaces, or operate entirely in the background. These are independent aspects of its design: an application does not need a user interface to be useful, and its underlying functionality should not be tightly coupled to a particular interface.

For example, an application might receive:
- A database capability scoped to a particular database or schema.
- A filesystem capability restricted to a particular directory.
- A messaging capability for communicating with other applications.
- An AI inference capability backed by a local model or remote service.
- A capability to invoke a typed interface exposed by another application.

It might also expose:
- A typed API consumed by other applications.
- A CLI for interacting with its functionality from a terminal.
- A web-based UI that can be hosted in a desktop WebView or regular browser.
- Background jobs or scheduled operations that require no direct user interaction.

The host determines which implementations satisfy the application's requirements and whether the application is authorized to use them. The platform provides the abstractions for connecting applications and users to the interfaces an application exposes.

This separates an application's functionality from both the infrastructure that provides its capabilities and the interfaces through which its functionality is accessed.

### 1.4 Application Interfaces

User interfaces are an essential part of the intended platform, even though implementing a complete UI hosting system is not an initial runtime milestone.

The platform should support multiple ways of interacting with an application without requiring its underlying functionality to be rewritten for each interface. A common application model should accommodate:
- Web interfaces: Applications can provide web-based frontends that run in a regular browser.
- Desktop interfaces: A host can present application frontends in embedded WebViews within a native desktop shell.
- Command-line interfaces: Applications can expose commands that invoke their typed functionality.
- Programmatic interfaces: Other applications and services can consume exposed functionality directly.
- Background execution: Applications can perform scheduled or event-driven work without a user-facing interface.

These interfaces should be treated as clients of application functionality, not as privileged extensions of the runtime. For example, a desktop WebView should not automatically inherit the capabilities granted to its application's backend. Instead, it should communicate through explicitly exposed interfaces, with the host mediating access where appropriate.

This separation allows different hosts to provide different presentation experiences while keeping application logic independent of the presentation technology. UI hosting can be implemented incrementally, but the architecture must account for interface communication and security boundaries from the beginning.

### 1.5 Modularity and Embeddability

The platform is intended to serve as a reusable foundation for different kinds of products and systems, rather than prescribing a single way of deploying or managing applications.

It should be possible to use the platform as a complete application environment or to incorporate selected components into a larger system that provides its own management interfaces, orchestration, and operational policies.

For example, a desktop host could use the platform to provide an application environment in which users install tools, inspect their requested capabilities, approve grants, and connect typed interfaces between applications. The desktop host provides the user experience; the platform supplies the application model, execution, capability, and composition infrastructure.

A server product could use the same platform to run persistent applications, provide remote capabilities, and manage application connections while supplying its own deployment, routing, monitoring, and operational policies. A larger system could also embed only the runtime or selected platform services rather than adopting the complete platform.

In particular:
- The runtime should be usable independently of the full platform.
- Platform management and capability configuration should be accessible programmatically.
- Hosts and management interfaces should be replaceable.
- Scheduling, routing, scaling, and other orchestration features should be optional components.
- Capability providers should be implementable independently and may be hosted locally or remotely.
- Components should have clearly defined responsibilities and interfaces so larger systems can reuse only the functionality they need.

Embeddability does not require all components to execute in the same process. An embedding system may interact with runtime workers through a supervisor or another defined interface. The platform's architecture should preserve these boundaries even when the initial implementation uses a simpler in-process arrangement.

### 1.6 Long-Term Direction

The long-term goal is a common application ecosystem spanning desktop, homelab, and cloud environments.

Applications should be composable without requiring them to share execution environments, credentials, or unrestricted system access. A desktop application might use a database hosted on a home server, an inference service running on a GPU machine, and a background worker running in the cloud—all through the same capability-oriented application model.

The same application functionality should be accessible through appropriate interfaces in different environments. A desktop host might provide a WebView, while a server host exposes a browser-accessible frontend and programmatic API. Background services can operate without either.

The platform should make it straightforward to create and deploy small applications while keeping their authority, dependencies, interfaces, and interactions understandable and manageable. As AI-assisted development makes creating applications increasingly inexpensive, the platform should make integrating and reusing those applications similarly straightforward.

The intended result is a modular foundation for building and composing software: one where application creation is flexible, authority is explicit, interfaces are reusable, and the underlying execution and composition model can be adopted by different products and environments.

### 1.7 Initial Hosts and Product Evolution

The first host is planned to be a desktop application that provides a graphical environment for managing and interacting with applications running on the platform. It will serve as the first complete user-facing demonstration and as a way to develop and use independent applications on the platform.

The desktop host is not the platform's defining abstraction. It should consume the platform through programmatic interfaces, so that the platform can later support a server host or be embedded in other products without depending on the desktop implementation.

The intended progression is:
1. Build the application runtime.
2. Build the core platform and its host-facing programmatic interfaces.
3. Build the first desktop host.
4. Develop and operate a collection of useful independent applications to validate the platform and its developer experience.
5. Extend the platform with a server host, remote capabilities, and synchronization where justified by real use cases.
6. Explore collaborative and broader third-party use, with a corresponding expansion of the security and operational requirements.

Each stage should produce a useful result and provide evidence for the next stage. Later stages are directions to validate, not commitments that must be implemented before the earlier stages are valuable.
## 2. Motivation and Goals

### 2.1 Making Application Authority Explicit

Modern application environments commonly provide applications with broad access to system resources. Depending on the runtime and deployment environment, an application may have access to the filesystem, network, environment variables, credentials, processes, and other resources simply because those facilities are available to it.

This creates a difficult security and operational problem: understanding what an application can actually do requires understanding not only its source code, but also the environment in which it executes and the resources available to it. This is particularly problematic when running independently developed, third-party, or AI-generated software.

The proposed platform addresses this by making application authority explicit. Applications receive capabilities from their host environment. These capabilities define the operations and resources available to them and can be inspected independently of the application's implementation.

Instead of asking what an application might be capable of doing with unrestricted access to its environment, an operator can ask what capabilities it has actually been granted. This does not eliminate the need to review application behavior. An application can misuse legitimate capabilities, expose information, or exploit vulnerabilities in trusted providers. However, it makes the application's available authority more understandable and manageable.

### 2.2 Isolation and Composition

Traditional application environments frequently rely on shared resources and implicit trust relationships for applications to communicate. Applications may share filesystems, databases, networks, credentials, or execution environments. While convenient, these arrangements make it difficult to establish meaningful boundaries and can make applications tightly coupled to one another's implementation details.

Strict isolation alone is not a sufficient solution. Applications need to share data, invoke services, and coordinate their behavior to build useful systems. The platform therefore treats isolation and interoperability as complementary requirements.

Applications are isolated by default, but can communicate through explicitly granted capabilities. An application can expose a typed interface, and another application can receive permission to invoke that interface without gaining access to the first application's internal resources.

For example, a notes application might own its database and expose a search interface. A dashboard application could receive permission to search notes without receiving direct database credentials or filesystem access.

The goal is to make sharing functionality easier than sharing unrestricted access to resources. This is both a security model and a composition model: independent applications should be straightforward to connect without bespoke integrations or shared implementation assumptions.

### 2.3 Enabling AI-Generated Software

AI-assisted software development is reducing the cost of creating applications. It is increasingly practical to generate small, purpose-built applications for individual workflows rather than relying exclusively on large, general-purpose software products.

However, the ability to generate code does not automatically solve the problems of reviewing, integrating, deploying, and maintaining it. Generated applications may depend on existing services, require access to sensitive resources, and introduce security or operational risks. They may also duplicate functionality that already exists in another application because there is no simple way to reuse it.

A capability-oriented platform provides a more manageable integration model. An AI agent can develop an application against defined interfaces, declare the capabilities it requires, and produce an application package. The platform can then identify its requirements, resolve them against available providers, and present requested permissions for review.

A possible workflow is:
1. An agent develops an application against documented platform and capability contracts.
2. The application declares the capabilities it requires and the interfaces it exposes.
3. The platform evaluates those requirements against available providers and host policy.
4. The host or operator authorizes appropriate grants and connections.
5. The platform runs the application with the resulting capabilities.

The platform should make it inexpensive to create applications while keeping their integration and operation understandable and controlled. It should not make generated software inherently trustworthy; it should provide a consistent environment in which software can be evaluated and operated with explicitly bounded authority.

### 2.4 Composable Software and Developer Experience

A central motivation for the project is to make independently developed applications easier to combine into larger systems.

Today, combining small applications often requires custom API integrations, shared credentials, direct database access, duplicated functionality, and deployment-specific configuration. Even when each application is individually useful, connecting them can become a separate engineering project.

The platform aims to make composition a normal part of application development:
- Applications expose typed interfaces that describe useful functionality.
- Other applications can declare dependencies on those interfaces.
- Hosts authorize connections between consumers and providers.
- Providers mediate access without exposing their internal implementation or credentials.
- Local and remote implementations can satisfy the same application-facing contract where appropriate.

This should allow an application to reuse functionality from a task manager, personal wiki, calendar, inference service, or other application without embedding that functionality or knowing how it is implemented. Applications can remain independently developed and updated while participating in a larger personal or collaborative software environment.

The platform's developer experience is therefore a core concern. Creating an application, declaring its needs, connecting it to existing functionality, and running it should be simpler than assembling the equivalent system manually from unrelated infrastructure components.

### 2.5 Desktop and Server Symmetry

Application development and deployment are often split across substantially different environments. Desktop applications may have access to local files, native interfaces, and locally installed services. Server applications typically operate through network APIs, remote databases, and managed infrastructure. Moving functionality between these environments can require significant changes to application code and deployment configuration.

The proposed platform aims to provide a common application model across desktop, homelab, and cloud environments. Applications should interact with resources through defined capabilities rather than depending directly on where those resources are implemented.

For example, an application might consume a database capability without knowing whether it is backed by a local SQLite database or a remote PostgreSQL service. An inference capability might be implemented by a local model, a GPU server on a home network, or a hosted API.

This symmetry should not imply that local and remote services behave identically. Network latency, availability, consistency, and failure modes remain important differences that application interfaces must account for. The goal is to preserve a common application model while allowing hosts to adapt implementations and operational policy to their environments.

### 2.6 Modularity and Embeddability

The platform is intended to serve as a reusable foundation for different kinds of products and systems, rather than prescribing a single way of deploying or managing applications.

A desktop host may use the complete platform to provide a personal application environment. A server host may use the same application model for persistent execution and remote services. An existing product may embed the runtime or selected platform services while supplying its own management experience, scheduler, routing, or scaling system.

This separation avoids requiring the project to implement every possible management or orchestration feature itself. It also allows the underlying application and capability model to be reused without adopting a particular product interface.

### 2.7 Project Goals

These motivations lead to the following high-level goals:

1. **Explicit authority:** Applications should only have access to resources and operations explicitly granted by their host.
2. **Default isolation:** Applications should execute independently, without inheriting ambient privileges or trusting other applications by default.
3. **Composable software:** Applications should be able to expose and consume typed functionality through explicitly authorized connections.
4. **Good developer experience:** Applications should be straightforward for developers and AI agents to create, test, integrate, and operate.
5. **Environment independence:** Application logic should not depend unnecessarily on whether resources are local, remote, desktop-hosted, or server-hosted.
6. **Interface independence:** Applications should support multiple interaction methods without coupling their underlying functionality to a particular presentation technology.
7. **Modularity and embeddability:** The runtime and platform services should be reusable independently and integrable into larger systems.
8. **Inspectability and manageability:** Hosts should be able to inspect applications, permissions, dependencies, interfaces, connections, and lifecycle state.
9. **Incremental evolution:** Each development stage should produce a useful system and validate the assumptions needed for subsequent stages.

Together, these goals define the intended direction of the project: a modular application platform that makes it easier to create, connect, and operate software without sacrificing control over what that software can access and do.
## 3. Core Concepts and Design Principles
The following principles are intended to guide architectural and implementation decisions throughout the project. They describe properties the system should preserve, rather than prescribing specific technologies or implementation details.
### 3.1 Explicit Authority
An application should only be able to access resources and perform operations through capabilities explicitly granted by its host.
The runtime should not expose ambient access to privileged system resources. Filesystem access, networking, databases, credentials, process management, and other privileged operations should be mediated by explicitly provided interfaces.
An application's effective authority should be inspectable without requiring a complete understanding of its source code.
Capability grants should be scoped as narrowly as practical. For example, an application might receive read-only access to a particular database schema rather than unrestricted database credentials.
The host should remain responsible for deciding which capabilities are available, regardless of what an application requests or declares.
### 3.2 Isolation by Default
Applications should be treated as independent and potentially untrusted units of execution.
One application should not automatically gain access to another application's memory, files, credentials, or internal services. Sharing should happen through explicitly exposed and granted interfaces.
Isolation should be enforced by the platform and its underlying execution environment, not merely by conventions followed by application developers.
The architecture should distinguish between logical application boundaries and the mechanisms used to enforce them. A logical application may be implemented using an isolate, process, container, or other execution mechanism, but the application model should not depend on a particular choice.
The security model should also distinguish between language-level isolation and operating-system-enforced isolation. Restrictions on JavaScript globals alone are not sufficient to establish a security boundary against malicious code.
### 3.3 Capability-Based Composition
Applications should compose through capabilities rather than unrestricted access to shared infrastructure.
An application may expose typed interfaces that other applications can consume. The host determines which applications are authorized to access those interfaces and supplies the appropriate capability implementations.
This should allow applications to share functionality without sharing their internal implementation or underlying resources.
For example, a database-owning application might expose a search interface. Other applications can invoke that interface without receiving direct access to its database.
Capabilities should be usable regardless of whether their implementations are local, remote, or provided by another application. The capability abstraction should describe the operations available to the consumer, while the host determines how those operations are implemented.
### 3.4 Host-Defined Environments
An application's execution environment should be determined by its host.
The application should not assume that particular system resources, services, or implementations are available simply because they exist in a particular runtime or operating system.
Instead, the host constructs the environment and supplies the capabilities that the application is permitted to use.
This allows the same application to run in different environments with different capabilities. A development environment might provide temporary filesystem access and debugging tools, while a production environment supplies restricted storage and access to specific services.
The application should interact with its environment through defined interfaces rather than depending on the implementation details of the host.
### 3.5 Interface and Presentation Independence
Application functionality should be independent of the interfaces through which it is accessed.
An application may expose functionality to other applications, provide a CLI, offer a web frontend, or run entirely in the background. These should be independent aspects of the application rather than tightly coupled implementations.
The platform should support multiple interaction mechanisms, including browser interfaces, desktop WebViews, command-line interfaces, and programmatic APIs.
User interfaces should be treated as clients of application functionality. They should not automatically inherit the application's privileged capabilities. Communication between a UI and its application should happen through explicitly exposed interfaces.
This principle should influence the design of the application model and runtime from the beginning, even though implementing complete UI hosting is outside the initial MVP.
### 3.6 Location Independence
Applications should consume capabilities without depending unnecessarily on where their implementations are hosted.
A capability may be provided by the local host, another application, a service on the local network, or a remote machine.
The platform should be able to connect applications to these providers without requiring the application to implement different integration logic for each deployment environment.
However, location independence should not hide important operational differences. Remote capabilities introduce latency, network failures, timeouts, partial failures, and other concerns that local implementations may not have.
The capability model should provide a consistent interface while preserving the ability to handle these differences explicitly.
### 3.7 Modularity and Embeddability
The platform itself should be composed of well-defined components with explicit responsibilities and interfaces.
The runtime, capability providers, application management services, UI hosting, and orchestration functionality should not be unnecessarily coupled.
A host system should be able to reuse the components it needs without being required to adopt the entire platform.
For example, a desktop application might embed the runtime and application management services while providing its own UI and installation workflow. A web orchestration system might use the runtime and capability infrastructure while providing its own scheduler, routing, and scaling mechanisms.
This means that the platform should expose programmatic interfaces for its core functionality. Management interfaces and orchestration policies should be replaceable rather than embedded into the runtime itself.
Modularity should not come at the expense of clear security boundaries. Components that enforce permissions or mediate privileged operations must have well-defined trust relationships and responsibilities.
### 3.8 Minimal Trusted Computing Base
The amount of software that must be trusted to enforce application security should be kept as small and understandable as practical.
Application code should be treated as untrusted. The runtime, supervisor, and capability providers occupy more privileged positions because they mediate execution and access to resources.
These privileged components should have narrowly defined responsibilities and explicit interfaces.
In particular, applications should not be able to bypass capability enforcement by accessing internal implementation details or privileged host APIs.
This principle favors a small trusted runtime and supervisor, with functionality implemented through separately defined providers wherever practical.
### 3.9 Inspectability and Manageability
The platform should make the structure and authority of the application environment understandable to its operators.
It should be possible to inspect:
- Which applications are installed and running.
- Which capabilities each application requests and receives.
- Which providers implement those capabilities.
- Which applications expose interfaces to other applications.
- Which applications are connected to one another.
- Where applications and their dependencies are running.
- What resources and operational constraints apply to each application.
This information should be available through programmatic interfaces so that different management tools can present it in useful ways.
The goal is not simply to enforce permissions, but to make the resulting system understandable enough that people can confidently manage and compose applications.
### 3.10 Separation of Policy and Mechanism
The platform should distinguish between the mechanisms that execute applications and enforce capabilities, and the policies that determine how applications are managed.
The runtime and supervisor are responsible for enforcing the authority granted to an application. Decisions about which applications should be deployed, which capabilities should be granted, where applications should run, and how resources should be allocated belong to the host or management system.
Different products may make different policy decisions while relying on the same underlying enforcement mechanisms.
For example, a desktop application manager might ask a user to approve each application's requested capabilities. A server orchestration system might apply centrally defined policies to automatically grant approved capabilities to deployed applications.
The underlying runtime should enforce the resulting grants consistently without needing to understand the policy that produced them.
This separation is important to making the platform reusable in different environments.

### 3.11 Security-Conscious Architecture and Incremental Hardening

Security boundaries should be part of the architecture from the beginning, but the level of security hardening should evolve with the platform's actual deployment model and threat environment.

Early development is expected to focus on a single-user environment in which the operator controls the host and applications are primarily created by agents operating under that user's direction. This narrower initial threat model makes it reasonable to prioritize clear trust boundaries, explicit capabilities, limited ambient authority, and a small privileged surface without attempting to deliver a hardened boundary against arbitrary malicious code in the first implementation.

This does not mean treating security as an afterthought. The architecture should preserve a path toward stronger enforcement:
- Application code and trusted host code should have clearly separated responsibilities.
- Applications should receive only explicitly granted capabilities.
- Privileged operations should be mediated by trusted host components.
- Inputs and outputs crossing trust boundaries should be validated at runtime.
- Application identity and authorization should not depend on a particular execution mechanism.
- Process and operating-system isolation should be introducible without changing application-facing capability contracts.

As the platform moves toward persistent server operation, remote access, sharing, and multiple users, its threat model and security requirements will expand. Stronger process isolation, operating-system enforcement, adversarial testing, and independent security evaluation should be introduced when those use cases justify them.

Early versions must describe their actual guarantees accurately. V8 isolates, API restrictions, and a security-conscious architecture are not equivalent to a hardened operating-system security boundary against malicious applications.
## 4. Proposed Architecture
### 4.1 Architectural Overview
The platform is organized into several layers with distinct responsibilities. The architecture separates application execution, security enforcement, capability implementation, application management, and the systems that host or embed the platform.
The major components are:
1. Application Runtime: Executes individual TypeScript applications within controlled environments.
2. Supervisor: Manages application execution and enforces capability grants and security boundaries.
3. Capability Providers: Implement the resources and services applications are permitted to access.
4. Platform Services: Provide higher-level application management, composition, service discovery, and lifecycle functionality.
5. Host Systems: Integrate the platform into a complete product or larger infrastructure system, providing user interfaces, orchestration, and operational policies.
These components should communicate through well-defined interfaces so they can be implemented, deployed, and replaced independently where appropriate.
The runtime and supervisor form the foundation of the system. Higher-level platform functionality builds on them but should not be required for basic application execution.
The architecture should support both a standalone platform and embedded deployments in which an external system uses selected components while providing its own management and orchestration functionality.
### 4.2 Application Runtime
The application runtime is responsible for executing application code within a controlled environment.
The initial implementation will focus on TypeScript and JavaScript applications, using V8 as the execution engine and an embeddable runtime foundation.
As the platform matures, the runtime is expected to provide:
- TypeScript transpilation and JavaScript execution.
- ES module loading and dependency resolution.
- A controlled mechanism for loading application dependencies.
- Host-injected globals and capability interfaces.
- Asynchronous execution and event-loop management.
- Useful error reporting, stack traces, and source maps.
- Support for multiple independent application instances.
- A mechanism for terminating and recovering application execution.
The runtime should not automatically expose ambient access to privileged system resources. Filesystem access, networking, process management, environment variables, and other sensitive functionality must be provided explicitly through host-controlled interfaces.
The initial implementation should prioritize a useful TypeScript developer experience while keeping the runtime's security model independent of a particular package manager or application development toolchain.
#### Proposed technology
The provisional technology choices are:
- Rust: Trusted host implementation and systems integration.
- V8: JavaScript execution engine.
- deno_core: Initial embedding foundation for V8, module infrastructure, asynchronous operations, and Rust integration.
- TypeScript: Application development language.
deno_core is attractive because it provides an embeddable foundation without requiring the project to adopt the complete Deno CLI or runtime architecture.
However, it does not provide the entire TypeScript development experience or npm compatibility by itself. Dependency resolution, TypeScript transpilation, package handling, and developer tooling will need to be assembled from appropriate components.
These technology choices are provisional. The architecture should depend on the capabilities and responsibilities of the runtime rather than becoming unnecessarily coupled to a particular implementation.
### 4.3 Supervisor and Security Boundary
The supervisor is trusted infrastructure responsible for managing application execution and mediating access to privileged resources.
It acts as the authority boundary between untrusted applications and the host environment.
Its responsibilities include:
- Maintaining application identities and execution configurations.
- Creating and terminating application instances.
- Constructing application environments.
- Enforcing granted capabilities.
- Mediating privileged operations.
- Managing application lifecycle.
- Enforcing resource limits.
- Providing controlled communication between applications and capability providers.
- Recording security-relevant events.
Applications should not be able to create their own privileged capabilities or bypass the supervisor to access host resources.
The supervisor should distinguish between requested capabilities and granted capabilities. Applications may declare what they need, but the host determines what they actually receive.
#### Isolation model
The architecture should support layered isolation.
At the language-runtime level, applications execute in controlled V8 environments with explicitly provided globals and APIs. This prevents applications from accidentally inheriting functionality that the runtime does not intend to expose.
However, JavaScript-level restrictions alone are not a sufficient security boundary against malicious code. Applications that are not fully trusted should eventually execute behind operating-system-enforced boundaries.
The intended direction is to support isolated runtime workers, potentially in separate restricted processes, communicating with the supervisor through narrow IPC interfaces.
The supervisor should retain privileged access to host resources while application workers receive only the communication channels and capabilities they require.
Operating-system mechanisms such as namespaces, filesystem restrictions, network restrictions, and resource controls can provide additional enforcement.
The exact isolation mechanism may vary by deployment environment, but the application security model should remain consistent.
### 4.4 Capability Providers
Capability providers implement the functionality applications are permitted to access.
A provider exposes a defined interface and enforces the authority granted to its consumers. Providers may be implemented as libraries, services, processes, or remote systems.
Examples include:
- Filesystem access.
- Database connections.
- HTTP clients and network access.
- Messaging and event delivery.
- AI inference.
- Application-to-application communication.
- Scheduling and other host services.
Providers should be independent of the applications consuming them and, where practical, independent of the runtime itself.
For example, a PostgreSQL provider might expose a typed database interface while retaining its connection credentials internally. An application receives permission to execute authorized operations through that interface without receiving unrestricted database credentials.
Providers may run locally, in separate processes, or remotely. The capability contract should describe the operations available to the consumer without unnecessarily exposing the provider's implementation details.
Providers are part of the trusted computing base for the operations they expose. They must correctly enforce their own access controls and must not assume that TypeScript types alone provide security.
### 4.5 Application Interfaces and UI Hosting
Applications should be able to expose functionality through multiple interaction mechanisms without coupling their underlying logic to a particular presentation technology.
The platform should accommodate:
- Typed programmatic interfaces consumed by other applications.
- Command-line interfaces.
- Browser-based frontends.
- Desktop frontends hosted in embedded WebViews.
- Background jobs and event-driven operations.
These interfaces should be treated as clients of application functionality rather than privileged extensions of the runtime.
A web frontend, for example, should communicate with its application through explicitly exposed interfaces. It should not automatically inherit the application's database, filesystem, or other backend capabilities.
The platform should eventually provide standardized mechanisms for discovering, connecting to, and hosting application interfaces.
UI hosting should remain a separate concern from the core application runtime. A desktop host may provide a native shell and WebView, while a server host may expose browser interfaces through HTTP.
Although a complete UI hosting implementation is outside the initial MVP, the application model, communication interfaces, and security boundaries must account for it from the beginning.
### 4.6 Platform Services and Application Composition
Platform services provide higher-level functionality for managing applications and their relationships.
They build on the runtime and supervisor rather than being intrinsic requirements of application execution.
Responsibilities may include:
- Application registration and identity.
- Application manifests and configuration.
- Capability requirement declarations.
- Capability grants and provider selection.
- Service discovery.
- Connections between applications.
- Application lifecycle and health management.
- Dependency management.
- Application versioning and updates.
- Inspection and audit interfaces.
An application should be able to declare its required capabilities independently of the policies used to grant them.
For example, an application might declare that it requires read-write access to a particular database interface and permission to invoke a search interface exposed by another application.
The platform resolves these requirements against available providers and host policy. The resulting grants determine the actual capabilities supplied to the application.
This separates three concerns:
1. Requirements: What an application says it needs.
2. Policy: What the host decides to permit.
3. Enforcement: What the runtime and providers actually allow.
The platform should expose these functions through programmatic interfaces so that standalone management tools and external host systems can use them.
### 4.7 Embedding and Orchestration
The architecture should support integration into larger systems without requiring those systems to adopt the platform's entire management model.
Two broad integration patterns are anticipated.
#### Embedded runtime
An external application incorporates the runtime and manages its application instances through programmatic interfaces.
The embedding system may provide its own:
- Application registry and installation workflow.
- User interface.
- Permission approval process.
- Capability providers.
- Application configuration.
- Lifecycle management.
For example, a desktop application could use the runtime to execute installed TypeScript applications while providing its own marketplace, WebView hosting, and permission-management experience.
#### Integrated platform
A larger system uses the platform's application management and capability abstractions while supplying additional infrastructure and operational policies.
For example, a web application orchestration platform could use the runtime to execute application instances while managing:
- Instance creation and termination.
- Scheduling and placement.
- HTTP routing and load balancing.
- Resource allocation.
- Scaling and recovery.
- Shared databases and other services.
- Monitoring and operational policy.
The runtime and supervisor should provide the underlying execution and authority enforcement mechanisms without requiring a particular orchestration strategy.
Distributed scheduling, routing, scaling, and high availability are not prerequisites for basic platform functionality. They should be introduced as optional higher-level components or integrations.
#### Process and deployment boundaries
Embeddability does not necessarily mean that all components must execute in the same process.
For security and reliability, an embedding system may host the supervisor in its own process and execute application workers in separate restricted processes. Communication can occur through explicitly defined IPC interfaces.
The architecture should support both in-process integration where appropriate and out-of-process execution where stronger isolation is required.
The important requirement is that external systems interact with stable interfaces rather than depending on the internal implementation of the runtime or supervisor.
### 4.8 Technology Choices and Rationale
The following table summarizes the current proposed technology direction.

| Component | Proposed technology | Rationale |
|---|---|---|
| Host and supervisor | Rust | Memory safety, systems integration, control over privileged operations |
| JavaScript engine | V8 | Mature, high-performance execution environment |
| Runtime foundation | `deno_core` | Embeddable V8 runtime, Rust operations, module infrastructure |
| Application language | TypeScript | Strong ecosystem, developer experience, AI coding support |
| Application isolation | V8 environments and OS sandboxing | Layered isolation and enforceable security boundaries |
| Privileged communication | IPC, mechanism to be determined | Narrow communication between trusted host and application workers |
| Capability interfaces | Typed contracts with runtime validation | Consistent application integration and explicit contracts |
| Application manifests | Declarative format | Express application identity, requirements, interfaces, and dependencies |
| Host configuration | Host-defined configuration; potentially TypeScript-based in a future host | Express host policy, orchestration, and product-specific behavior |
These choices are starting points rather than immutable requirements.
In particular, the initial runtime prototype should validate whether deno_core provides an appropriate foundation for the desired application model before significant platform functionality is built around it.
The implementation should favor well-defined interfaces and replaceable components over unnecessary dependencies on particular frameworks or infrastructure products.

## 5. Application Model
This section defines the conceptual model of an application and how applications declare their requirements, receive capabilities, and operate within the platform. It establishes the relationships between application identity, manifests, capability declarations, authorization, execution instances, and persistent state.
The goal is to define a consistent model that can support independently developed applications without requiring the platform to prescribe how every application is implemented.
### 5.1 Application Identity and Structure
An application is a logical entity with a stable identity, independent of its implementation, version, or running instances.
An application consists of several related elements:
- Identity: A stable identifier used to distinguish the application within the platform.
- Manifest: A declarative description of the application, including its metadata, entry points, dependencies, and capability requirements.
- Code: The executable implementation of the application.
- Dependencies: Other packages, libraries, or resources required by the application.
- Configuration: Host-provided settings and application-specific configuration.
- Persistent State: Data maintained by the application across execution instances and restarts.
An application may have multiple versions and multiple running instances. These are distinct from the application's logical identity.
The application model should not assume that an application corresponds to a single process, container, or other particular execution unit. The platform determines how applications are instantiated and isolated.
### 5.2 Application Manifests
Every application should have a manifest that describes its identity, structure, and requirements to the host platform.
The manifest provides the information necessary for the host to understand an application before executing it. It should be declarative rather than a general-purpose programming environment.
At a conceptual level, the manifest describes:
- Application identity and metadata.
- Version information.
- Application entry points.
- Dependencies.
- Required startup capabilities.
- Optional startup capabilities.
- Capabilities that may be requested dynamically during execution.
- Other application-specific declarations or configuration requirements.
The manifest should provide a general mechanism for communicating structured declarations to the host. The platform should not need to understand the semantics of every possible capability declaration. Those semantics can be defined by the relevant host services and capability providers.
The specific manifest format, schema, and validation rules will be established during implementation.
### 5.3 Capability Requirements
Applications declare the capabilities they require through their manifests.
A capability declaration communicates what functionality or authority an application expects the host to provide. Declarations may range from simple identifiers, such as database.read, to structured objects containing additional information about the requested capability.
Capabilities may be classified as:
- Required: The application cannot operate as intended without the capability. The host must satisfy the requirement before starting the application.
- Optional: The application can operate without the capability, although some functionality may be unavailable.
- Dynamically Requested: The application may request the capability during execution rather than requiring it at startup.
Applications are developed against the capabilities provided by their intended host environment. The platform does not attempt to make every application compatible with every possible host. Instead, it makes application requirements explicit so that hosts can determine whether they can satisfy them.
Capability declarations describe application requirements, not authorization decisions. Declaring a capability does not grant access to it.
### 5.4 Capability Authorization and Injection
The host is responsible for determining which requested capabilities an application is authorized to receive.
Before execution, the host evaluates the application's manifest against its available capabilities and authorization policies.
The host then constructs the application's execution environment, injecting the capabilities that have been authorized.
For example, an application might declare a requirement for filesystem access. A desktop host could provide a filesystem capability backed by local storage, while a server host could provide an implementation backed by a restricted directory or another storage system.
The application interacts with the capability through its defined interface without needing to know how the host implements it.
The following principles govern capability authorization:
- Applications cannot grant themselves capabilities.
- The host determines which capabilities are available to each application.
- Required capabilities must be satisfied before execution.
- Optional capabilities may be omitted.
- Unauthorized capabilities are not injected.
- Capability providers enforce the authority represented by their interfaces.
The platform does not negotiate application requirements or silently reduce requested permissions. If a host cannot satisfy an application's required capabilities, it should refuse to start the application. The application developer is responsible for determining which capabilities are required, which are optional, and how the application behaves when optional functionality is unavailable.
### 5.5 Dynamic Capability Requests
Some applications need access to capabilities that cannot reasonably be determined entirely at startup.
For example, a desktop application might allow a user to select a directory and subsequently request access to that directory. A server application might have all its required capabilities provisioned before execution and never need to request additional authority.
Applications should be able to declare which capabilities they may request dynamically.
When an application requests an additional capability, the host evaluates the request against its authorization policy. Depending on the host environment, this could involve user approval, administrative policy, or automatic authorization.
Dynamic capability requests do not provide a mechanism for applications to bypass authorization. Every request must be explicitly authorized by the host before the capability becomes available.
Hosts may choose not to support dynamic requests or may impose policies that prohibit applications requiring them. This allows desktop and server environments to support different authorization models without requiring different application execution architectures.
The details of dynamic request handling, including its API and interaction with running application instances, remain implementation decisions.
### 5.6 Capability Providers and Implementations
A capability is an interface through which an application accesses functionality or resources provided by its host environment.
The implementation behind a capability is provided by a capability provider. Providers may be implemented as platform services, independent components, or adapters to external systems.
Applications should depend on capability contracts rather than particular provider implementations.
For example, a database capability might be implemented using local SQLite, a remote PostgreSQL server, or another compatible database service. The host selects and configures the implementation.
Capability providers are responsible for enforcing the authority represented by their interfaces. TypeScript interfaces and application-level conventions are not security boundaries. Providers must validate operations and enforce applicable restrictions at runtime.
The capability model should be extensible enough to support capabilities implemented by components outside the core platform.
This also preserves the possibility of applications eventually providing capabilities to other applications. First-class application-provided services, discovery, and connection management are deferred until a concrete use case justifies them.
### 5.7 Application Instances and Lifecycle
An application identity is distinct from its running instances.
A single installed application may have zero or more running instances. Each instance represents an execution of the application within a particular environment.
Instances have their own:
- Execution context.
- Granted capabilities.
- Runtime configuration.
- Resource allocations and limits.
- Lifecycle state.
The platform manages application instances independently of the logical application identity.
An application may be installed without currently running, stopped and restarted, or instantiated multiple times where the host permits it.
The platform should support lifecycle operations such as starting, stopping, terminating, and restarting instances. It should also track execution failures and provide mechanisms for monitoring instance health.
The specific lifecycle states and instance management APIs will be defined during implementation.
### 5.8 Configuration, Secrets, and Persistent State
Application configuration, secrets, and persistent state are distinct concerns and should be handled through explicit mechanisms.
Configuration consists of settings supplied by the host or application administrator. Configuration may vary between environments and instances.
Secrets include credentials, tokens, and other sensitive values. They should be provided through narrowly scoped capabilities rather than embedded in application packages or exposed through unrestricted environment variables.
Persistent State consists of data that must survive application restarts and potentially changes to execution instances. Access to persistent storage should be explicitly provisioned and scoped rather than assumed to exist as ambient filesystem access.
The platform should not assume that application state is tied to a particular execution instance or physical storage location. Hosts may provide local or remote persistence implementations.
The details of state provisioning, migration, backup, and recovery are implementation concerns, but the application model should preserve the distinction between executable code, configuration, secrets, and persistent data.
### 5.9 Application Composition and Future Extensions
The capability model provides the foundation for applications to interact with platform services and, potentially, other applications.
Applications consume capabilities through interfaces injected by their host. The host controls which providers are available and which applications are authorized to use them.
This model is sufficient for the initial platform. A dedicated application-to-application service model is not required to establish the core execution and security architecture.
Future extensions may allow applications to expose typed interfaces that other applications can consume. Such interfaces could be made available through the same underlying capability mechanism, preserving a consistent authorization and enforcement model.
The initial implementation should avoid introducing a separate application connection or service-discovery system before those capabilities are needed.
The central principle is that application composition should build upon the existing capability architecture rather than introduce a parallel mechanism for granting authority.
## 6. Capability Model
This section defines how applications access functionality and resources provided by their host environment. Capabilities are the fundamental mechanism through which applications receive authority.
The capability model provides a consistent interface between applications and their environment while allowing the host to control authorization, implementation, and execution.
### 6.1 Capability Representation
A capability is an object injected into an application's execution environment that provides access to a related set of operations.
Applications access capabilities through a root object named host. Capabilities are organized hierarchically beneath this object.
For example:
```ts
const contents = await host.files.readText("hello.txt");

await host.database.query("SELECT * FROM notes");

const response = await host.inference.generate({
  model: "some-model",
  prompt: "Summarize this document"
});
```
The host object is the application's entry point to host-provided functionality. Its constituent capabilities represent distinct interfaces and grants of authority.
The root object is not itself a single permission. The host determines which individual capabilities are made available to each application.
Capabilities may expose multiple related operations, and the host may provide different implementations of the same capability contract.
### 6.2 Capability Contracts
Each capability is defined by a contract describing the operations it exposes, their inputs and outputs, and their expected behavior.
TypeScript interfaces provide the primary developer-facing representation of these contracts.
For example:
```ts
interface FilesystemCapability {
  readText(path: string): Promise<string>;
  writeText(path: string, contents: string): Promise<void>;
}
```
These contracts provide a consistent programming interface for application developers and capability providers.
TypeScript types are not security enforcement mechanisms. Types are erased at runtime, and values crossing execution or process boundaries must be validated by trusted components.
Capability providers must validate inputs and enforce their contracts at runtime.
### 6.3 Capability Versioning
Capability contracts should have explicit versions.
Applications declare the capability versions they require, and the host determines whether it can provide compatible implementations.
For example:
```ts
{
  capabilities: {
    "host.files@1": {
      required: true
    },
    "host.database@2": {
      required: true
    }
  }
}
```
If the host does not support a required capability version, it cannot satisfy the application's requirements and should refuse to start the application.
Optional capabilities may be omitted when unavailable.
Provider implementation versions are distinct from capability contract versions. The host may update a provider's implementation without changing the capability version, provided the implementation continues to honor the existing contract.
The specific versioning scheme and compatibility rules will be established during implementation.
### 6.4 Capability Scope and Authority
The authority represented by a capability is determined by its implementation and the resources to which that implementation has access.
Applications may use the same capability contract while receiving different underlying authority.
For example, two applications might both receive a filesystem capability with the same interface, but one may be restricted to its own data directory while another has access to a shared project directory.
The provider enforces these restrictions. An application cannot expand its authority simply by supplying different arguments to capability operations.
Capability contracts should remain relatively stable while the host determines the resources and permissions associated with each injected capability.
### 6.5 Capability Providers
A capability provider is a trusted component that implements a capability contract and makes its functionality available to applications.
Providers may be implemented in different ways:
- As components running within the host or supervisor process.
- As separate processes.
- As adapters to external services.
- As interfaces to remote systems.
The application interacts with the capability contract rather than the provider's implementation details.
For the initial implementation, capability providers are trusted components of the host environment. Their dependencies and authority are managed by trusted host code rather than through a separate provider-level capability system.
An application authorized to use a provider's capability is trusted to use the functionality exposed by that capability. The provider is responsible for honoring its contract and enforcing the application's applicable restrictions.
This is a deliberate simplification of the initial architecture. More granular provider isolation and authority management may be introduced later if required.
### 6.6 Capability Invocation and Communication
All application-facing capability operations should use asynchronous APIs.
For example:
```ts
const contents = await host.files.readText("hello.txt");

await host.files.writeText("hello.txt", contents);
```
Even when a capability is implemented locally and can perform an operation synchronously, its application-facing interface should remain asynchronous.
This provides a consistent programming model regardless of whether a capability is implemented locally, in another process, or remotely.
Capability invocation may be implemented using different underlying mechanisms:
- Direct calls to host components.
- V8 host functions and objects.
- Inter-process communication.
- Network communication with remote providers.
These implementation differences should not require changes to the application's capability contract.
However, the platform must preserve the operational differences between local and remote capabilities. Network latency, timeouts, connection failures, and provider unavailability remain meaningful application concerns.
Capability interfaces may also support asynchronous streams, subscriptions, and other long-running interactions where appropriate.
### 6.7 Capability Lifecycle and Revocation
Capabilities may become unavailable during application execution due to authorization changes, provider failures, or other lifecycle events.
The primary enforcement mechanism is the provider itself. Every operation must be subject to authorization and availability checks, regardless of whether the application still holds a reference to the injected capability object.
If access is withdrawn, subsequent operations must be rejected by the provider.
Applications are responsible for handling authorization failures and other capability errors.
The initial implementation does not require a sophisticated revocation protocol, live permission-change notifications, or automatic cancellation of outstanding operations. These may be introduced later if practical experience demonstrates a need.
### 6.8 Capability Extensibility
The capability model should not be limited to a fixed collection of built-in capabilities.
The architecture should permit additional capability types and implementations to be introduced without requiring changes to the core runtime.
Applications target the capabilities documented by their intended host environment. Runtime discovery of arbitrary capabilities and automatic provider selection are not required for the initial implementation.
The host determines which capability implementations are available and which applications are authorized to use them.
A formal third-party provider plugin system may be introduced later, but is not a prerequisite for the initial architecture.
### 6.9 Design Summary
The capability model establishes the following principles:
- Capabilities are injected objects representing access to functionality or resources.
- Applications access host-provided functionality through the host root object.
- Capability contracts are defined using TypeScript interfaces and explicit versions.
- The host determines which capabilities are available to each application.
- Capability providers enforce the authority and restrictions associated with their implementations.
- Providers are trusted components of the initial host architecture.
- Application-facing capability operations are asynchronous.
- Capability implementations may be local, process-isolated, or remote.
- Capability providers validate inputs and enforce their contracts at runtime.
- The architecture supports future extensibility without requiring a formal provider plugin system or provider-level isolation in the initial implementation.
The capability model is intended to provide a simple, consistent foundation for application access to host functionality while preserving the platform's central security principle: applications receive only the authority explicitly provided to them by their host.

## 7. Runtime Execution and Isolation
This section describes how applications are executed, how their execution environments are constructed, and how the runtime evolves from an initial proof of concept into a more robust isolated execution platform.
The initial implementation prioritizes proving the core application model: executing TypeScript applications in controlled environments with explicitly injected capabilities. More advanced process supervision and operating-system isolation will follow.
### 7.1 Execution Model
Each application instance executes inside its own V8 isolate.
An isolate provides an independent JavaScript execution environment, including its own global scope, module state, and execution context. Multiple isolates can exist within a single host process, but applications cannot directly access one another's execution environments.
Applications communicate and share resources exclusively through capabilities provided by the host.
The logical application instance is independent of its eventual implementation unit. An instance may initially execute inside an isolate in the host process, but later execute inside a separate worker process or operating-system sandbox without changing its application-facing interfaces.
A V8 isolate provides execution-context separation, but it is not considered a sufficient security boundary against malicious code. Stronger isolation will be introduced in later implementation stages.
### 7.2 Runtime Implementation
The initial runtime will be implemented in Rust using V8 through deno_core.
The runtime is responsible for:
- Initializing and managing V8.
- Creating and disposing of application isolates.
- Loading application modules.
- Transpiling TypeScript into executable JavaScript.
- Constructing application execution environments.
- Injecting authorized host capabilities.
- Executing application entry points.
- Driving asynchronous execution.
- Reporting execution errors and termination.
The project will use components from the Deno ecosystem rather than adopting the complete Deno runtime.
This allows the project to reuse mature JavaScript execution infrastructure while retaining control over available APIs, module loading, application environments, and capability injection.
### 7.3 TypeScript Transpilation
Applications will be authored in TypeScript, but V8 executes JavaScript.
The runtime must therefore transpile TypeScript source into JavaScript before execution.
The initial implementation will support:
- TypeScript source files.
- Standard JavaScript-compatible TypeScript syntax.
- ES modules.
- Relative module imports.
- Top-level await.
TypeScript transpilation is distinct from type checking. Transpilation removes TypeScript-specific syntax so the resulting JavaScript can execute. Full TypeScript type checking is a separate development-time concern and is not required for the initial runtime.
deno_ast is a candidate for providing TypeScript transpilation.
Advanced compilation workflows and integrated type checking are deferred.
### 7.4 Module Loading and Imports
Applications will use native JavaScript ES module semantics.
The runtime will provide a controlled module loader that resolves application imports within the application's defined source directory.
The initial implementation will support relative imports between application files, including TypeScript modules.
For example:
```ts
// main.ts
import { greet } from "./greeting.ts";

console.log(greet("world"));
```
The module loader must prevent imports from escaping the application's source directory or accessing arbitrary external modules. Path normalization and symlink handling must be considered so that relative paths cannot bypass the source-directory boundary.
The initial implementation will not support:
- Arbitrary network imports.
- npm package resolution.
- General Node.js compatibility.
- Unrestricted access to the host filesystem through module loading.
These features can be evaluated later without changing the fundamental application execution model.
### 7.5 Host Capability Injection
Applications receive access to resources and functionality through explicitly injected capabilities.
The application-facing API will be exposed through a host-provided namespace:
```ts
const contents = await host.files.readText("hello.txt");
```
The runtime will not expose general-purpose operating-system APIs through Node.js or the full Deno runtime. In particular, applications will not receive ambient filesystem, process, or network access.
Instead, the Rust host implements the underlying operations and exposes narrow JavaScript bindings to the application.
The execution flow is:
1. The host determines which capabilities are authorized for the application instance.
2. The runtime constructs the application's V8 environment.
3. Authorized capability interfaces are injected into that environment.
4. Application calls are dispatched through host-controlled bindings.
5. Trusted host implementations perform the requested operations and return results.
The authority of a capability resides in its host-controlled implementation and associated authorization state, not in the JavaScript object representing it.
An application may construct or modify JavaScript objects that resemble capability interfaces, but doing so does not create authority or grant access to host resources. Only genuine host-controlled bindings can perform privileged operations.
This distinction must remain true as the implementation evolves from direct in-process calls to IPC between separate processes.
### 7.6 Asynchronous Execution
All application-facing capability operations will use asynchronous interfaces, even when the underlying implementation performs a synchronous operation.
For example:
```ts
const contents = await host.files.readText("hello.txt");
```
This provides a consistent programming model across local implementations, separate processes, and remote providers.
The runtime must support:
- JavaScript promises.
- async and await.
- Top-level await.
- Asynchronous host operations.
- Promise rejection and error propagation.
- Event-loop progression while asynchronous operations are pending.
deno_core provides useful infrastructure for managing V8 execution and asynchronous operations.
More advanced functionality, such as timers, long-running applications, and subscriptions, can be introduced as the application model develops.
### 7.7 Application Lifecycle and Error Handling
The initial execution lifecycle is intentionally simple:
1. Create an application isolate.
2. Construct its execution environment.
3. Inject authorized capabilities.
4. Load the application entry point and dependencies.
5. Execute the application.
6. Drive the event loop until execution completes or is terminated.
7. Capture and report exceptions and unhandled promise rejections.
8. Dispose of the isolate.
The host must be able to terminate execution explicitly.
The initial implementation does not require sophisticated resource quotas, automatic recovery, scheduling, or process supervision. Those responsibilities will become increasingly important as the platform begins managing persistent and concurrently running applications.
### 7.8 Isolation and Implementation Stages
Isolation will be developed incrementally.
Stage 1: Isolate-based execution
The initial implementation runs the Rust host and application runtime in the same operating-system process. Each application instance receives its own V8 isolate and explicitly injected capabilities.
This stage proves that the platform can execute TypeScript applications in controlled environments without granting ambient operating-system access.
It is a runtime proof of concept, not a security boundary suitable for executing arbitrary malicious code. It does not attempt to defend against vulnerabilities in V8 or the trusted Rust host.
Stage 2: Separate application worker processes
The host evolves into a supervisor capable of managing application instances in separate worker processes.
The supervisor becomes responsible for creating, monitoring, communicating with, and terminating workers. This improves lifecycle management and fault containment, but process separation alone does not provide a robust security boundary if workers retain the same operating-system identity and privileges. Capability operations cross the process boundary through a controlled IPC mechanism.
The application-facing capability interfaces should remain unchanged. An application should not need to know whether an operation is handled directly in the host process or dispatched to a separate worker or provider.
Stage 3: Operating-system isolation
Application workers are placed inside restricted operating-system environments.
Depending on the target environment, this may include:
- Restricted operating-system users.
- Filesystem isolation and namespace restrictions.
- Network access restrictions.
- System-call filtering.
- Resource limits.
- Additional process-level security controls.
The objective is to establish an actual operating-system security boundary around application execution.
The exact isolation mechanisms will depend on the deployment environment and will be designed in a later stage.
## 8. Initial Implementation Plan

Development should proceed from the runtime foundation toward a usable platform, then toward concrete hosts and applications that validate the platform's value. The runtime is the first implementation step, but the platform—not the runtime or a particular desktop interface—is the primary product.

The sequence is intentionally incremental. Each stage should produce a useful, demonstrable result and expose architectural assumptions to real use before the project commits to broader functionality.

### 8.1 Development Sequence

The intended progression is:

1. **Runtime foundation:** Build a Rust-based TypeScript runtime using V8 and `deno_core`, with controlled module loading and host-injected capabilities.
2. **Core platform and host API:** Build the application model and the trusted host services needed to register, configure, authorize, execute, inspect, and connect applications.
3. **Desktop host:** Build a graphical host that uses the platform to manage applications, inspect and approve capabilities, configure connections, and provide application interfaces.
4. **Independent applications and dogfooding:** Build useful applications on the platform, use them in real workflows, and improve the application model and developer experience based on actual integration needs.
5. **Server host:** Extend the platform toward persistent server execution, remote capabilities, and synchronization where justified by the applications and workflows developed so far.
6. **Collaborative and broader use:** Explore sharing and multi-user operation, with a corresponding expansion of identity, policy, isolation, and security requirements.

This sequence describes development priorities, not a requirement that every stage be completed before useful work can happen. The first runtime prototype can be small; the first platform host can be a CLI or test harness; and the desktop host can grow alongside the platform once its programmatic boundary is sufficiently clear.

### 8.2 Runtime MVP Objective

The first implementation milestone is a small Rust-based runtime capable of:
- Initializing V8 through `deno_core`.
- Loading and executing TypeScript applications.
- Supporting ES modules and relative imports.
- Creating a separate V8 isolate for each application instance.
- Injecting explicitly authorized host capabilities.
- Dispatching capability operations to trusted Rust implementations.
- Supporting asynchronous capability calls and top-level await.
- Capturing execution errors and terminating applications.

The runtime MVP should demonstrate that application functionality can execute in a controlled environment and access host resources only through explicitly injected interfaces. It is a foundation for the platform, not a complete application management product or a hardened security environment.

### 8.3 Initial Capabilities

The first implementation should use a small number of capabilities to validate the injection and dispatch model.

A filesystem capability is a useful first example because it demonstrates how an application can interact with host resources without receiving general filesystem access. A second simple capability should demonstrate that the runtime supports multiple independently implemented interfaces.

The initial capability set should remain small. The goal is to validate the architecture and application-facing contract, not to build a comprehensive provider ecosystem.

### 8.4 Runtime Development Milestones

**Milestone 1: Basic TypeScript execution**
- Initialize V8 through `deno_core`.
- Transpile and execute a TypeScript entry point.
- Support ES modules and relative imports.
- Support top-level await.
- Capture execution errors.

**Milestone 2: Capability injection**
- Define the application-facing `host` namespace.
- Define TypeScript contracts for initial capabilities.
- Implement host-controlled JavaScript bindings.
- Dispatch operations to Rust implementations.
- Ensure application-created JavaScript objects cannot acquire host authority.
- Validate inputs and outputs at the trusted boundary.

**Milestone 3: Application instances**
- Establish the application-instance execution model.
- Create one isolate per instance.
- Associate capabilities and configuration with individual instances.
- Ensure instances cannot directly access one another's execution contexts.
- Support explicit termination and cleanup.

**Milestone 4: Runtime demonstration**

Build a small application that uses multiple injected capabilities to demonstrate that application logic is independent of the underlying host implementations. Make it possible to inspect the application's source, capability requirements, and capabilities actually provided to it.

### 8.5 Core Platform and Host API

After the runtime foundation is demonstrated, build the smallest useful platform layer around it. This stage should establish the distinction between application requirements, host policy, authorization decisions, and enforcement.

Initial platform responsibilities should include:
- Application identity and registration.
- Declarative application manifests and validation.
- Application version and instance metadata sufficient for the initial host.
- Capability contract definitions and provider registration.
- Evaluation of application requirements against host-provided capabilities.
- Recording and applying authorized grants.
- Construction of application environments using those grants.
- Basic application lifecycle operations and status inspection.
- A programmatic interface through which a host can perform these operations.

The first host-facing API does not need to be a fully generalized remote management protocol. It should, however, establish a deliberate boundary between platform mechanisms and host-specific presentation or policy. The API can evolve as the desktop and server use cases become concrete.

Application-to-application composition should be demonstrated early enough to validate the central thesis. At minimum, the platform should support an application exposing a typed interface and another application receiving an explicitly authorized connection to it. The first implementation can use a simple local transport; the application-facing contract should not depend on that transport.

### 8.6 Desktop Host

The first complete host should be a desktop application that consumes the platform through its programmatic interfaces.

Its initial responsibilities should focus on making the platform usable:
- Display installed applications and their status.
- Install or register locally developed applications.
- Start, stop, and inspect application instances.
- Display declared capability requirements and actual grants.
- Allow the operator to configure capability providers and approve grants.
- Configure connections between application interfaces.
- Provide a way to open or interact with application user interfaces where supported.
- Surface application logs and execution errors.

The desktop host is both a user-facing product and a practical test of the platform API. It should not absorb core runtime or authorization responsibilities that belong in the platform. Its interface and presentation choices should remain replaceable.

### 8.7 Application Development and Dogfooding

Once the desktop host can manage applications, build a collection of independent applications that solve useful problems and exercise different parts of the platform.

These applications should be used to test whether the platform makes software easier to create, integrate, and maintain. Useful validation cases include:
- An application consuming a shared data or storage capability.
- An application exposing a typed interface to another application.
- Two applications composed through an explicitly authorized connection.
- An application using a provider whose implementation can be changed without changing application logic.
- Applications with different user interfaces or no user interface.

The applications should remain independent products rather than becoming modules of one large application. Friction encountered while building and connecting them should inform changes to manifests, capability contracts, tooling, lifecycle management, and host APIs.

### 8.8 Server Host and Later Stages

A server host should be pursued after the desktop environment and independent applications demonstrate a useful platform model. It should extend the same application abstractions toward persistent execution and remote operation rather than introduce a separate application model.

Likely areas of work include:
- Persistent application instances and process supervision.
- Remote capability providers and application connections.
- Deployment and configuration for a headless host.
- Data persistence, backup, and synchronization.
- Cross-device access to applications and their interfaces.
- Monitoring, recovery, and resource management.

Collaborative use with multiple people introduces additional concerns around identity, ownership, sharing, authorization policy, isolation, and trust. These should be treated as a meaningful expansion of the threat model rather than as a minor extension of the single-user desktop environment.

### 8.9 Security Development Stages

Security-conscious architecture is required from the beginning, but security hardening should be staged according to the threat model.

**Initial single-user development:** Maintain explicit capability boundaries, trusted-host separation, runtime validation, and a small privileged surface. The initial in-process V8 implementation is a development foundation, not a security boundary against arbitrary malicious code.

**Process separation:** Introduce separate application worker processes and controlled IPC as lifecycle management and fault containment become necessary. Process separation improves operational isolation but does not, by itself, establish a robust boundary against malicious code, especially when workers share the same operating-system user and privileges.

**Operating-system enforcement:** Add restricted users, filesystem and network restrictions, namespaces, syscall filtering, resource limits, or platform-appropriate equivalents when stronger isolation is required by server or collaborative use cases.

**Security evaluation:** Before making strong security claims or supporting higher-risk multi-user deployments, develop a concrete threat model, perform adversarial testing, review the trusted computing base, and obtain independent security evaluation as appropriate.

### 8.10 Success Criteria

The project should be evaluated at each stage rather than against a single all-encompassing MVP definition.

The runtime foundation is successful when:
1. A TypeScript application can execute inside a V8 isolate managed by the Rust host.
2. The application can import other local TypeScript modules.
3. The application can use explicitly injected capabilities.
4. Capability operations are implemented and authorized by trusted host code.
5. Applications cannot gain host authority by constructing or modifying JavaScript objects.
6. Separate application instances have separate execution contexts and capability state.
7. Asynchronous capability calls work correctly.
8. The host can observe execution errors and terminate an application.

The core platform is successful when a host can register and inspect applications, evaluate their declared requirements, apply grants, manage instances, and establish at least one authorized application-to-application connection through typed interfaces.

The desktop host is successful when it can manage and interact with several independently developed applications without containing their application logic or bypassing the platform's authority model.

The dogfooding stage is successful when building and composing real applications reveals that the platform materially reduces the effort required to integrate functionality and manage application authority compared with ad hoc integrations.

None of these early success criteria imply a complete security boundary against malicious code. Process-level and operating-system enforcement are subsequent objectives.
## 9. Future Architectural Decisions

The preceding sections establish the initial architecture and development sequence, but several important areas require further design work. These decisions should be made when there is enough implementation experience or a concrete use case to evaluate the tradeoffs. Deferring them should not prevent progress, but the architecture should avoid unnecessarily foreclosing reasonable options.

### 9.1 Platform API and Host Architecture

The platform should expose a programmatic interface that allows host applications to configure and manage it. The exact shape and stability guarantees of that interface can evolve, but the architectural separation between platform mechanisms and host-specific behavior is required from the beginning.

A future TypeScript host layer may consume the Rust platform API, allowing host-specific configuration, policy, orchestration, and user experiences to be implemented in TypeScript while retaining Rust-based enforcement.

Questions to resolve:
- What operations belong in the platform API?
- Which operations require administrative authority?
- How should TypeScript communicate with the Rust platform: native bindings, IPC, or another interface?
- Should the API support multiple clients and remote management?
- How should host extensions be loaded and granted authority?
- How should host configuration and policy be represented?
- Which platform operations must remain available when the desktop UI is replaced?

### 9.2 Host Responsibilities and Different Environments

The first host is a desktop application, but the platform should not be designed around desktop-specific assumptions. The architecture should be tested against at least three host environments:
- **Desktop host:** User-facing application installation, permission approval, capability connections, application dashboards, and WebView-based interfaces.
- **Server host:** Administrator-defined policies, automated provisioning, persistent application execution, remote capabilities, synchronization, and remote management.
- **Embedded host:** A larger application that reuses the runtime or platform services while supplying its own management experience and orchestration.

Questions to resolve:
- Which responsibilities are platform mechanisms and which belong to the host?
- Which policies are supplied by a user, administrator, or embedding application?
- How are host-specific capabilities and interfaces declared?
- What is the minimum host API needed by each environment?
- How can different hosts provide different experiences without fragmenting the application model?

### 9.3 Application Composition and Connection Model

Application composition is central to the platform's purpose and should be validated early, even if a complete discovery or provider ecosystem is deferred.

Questions to resolve:
- How does an application declare the interfaces it exposes?
- How does a consumer declare dependencies on another application's interface?
- How does a host discover compatible providers?
- How are connections authorized, configured, inspected, and revoked?
- How are interface versions and compatibility handled?
- How are local and remote connections represented?
- What failure, timeout, and availability semantics are visible to consumers?
- Can an application expose multiple interfaces with different scopes or audiences?

### 9.4 Process Supervision and Inter-Process Communication

Separate application worker processes are a likely next step as persistent and concurrent application execution becomes important.

Questions to resolve:
- Worker process creation and termination.
- IPC protocol and message formats.
- Capability dispatch across process boundaries.
- Failure detection and recovery.
- Application startup and shutdown semantics.
- Resource accounting and limits.
- Whether workers are grouped or isolated individually.
- How the supervisor is updated and kept separate from application-controlled code.

The application-facing capability contracts should remain independent of the transport mechanism. Process separation should be understood as an operational and fault-containment improvement, not as a complete security boundary by itself.

### 9.5 Operating-System Isolation and Security Evaluation

The platform needs a concrete security model before it makes strong claims about executing hostile applications or supporting higher-risk multi-user deployments.

Questions to resolve:
- Which operating-system isolation mechanisms are supported?
- How are filesystem and network restrictions represented and enforced?
- How are resource limits enforced?
- How is the trusted computing base minimized?
- How are isolation mechanisms made portable across Linux, macOS, and other environments?
- What security guarantees can each deployment environment realistically provide?
- What threat models apply to single-user, server, and collaborative deployments?
- What adversarial testing and independent review are required before making particular security claims?

The design must distinguish between API-level restrictions, V8 isolate separation, process separation, and operating-system security boundaries.

### 9.6 Application Packaging and Distribution

Questions to resolve:
- How are applications packaged and distributed?
- How are dependencies declared, resolved, and stored?
- How are application versions and updates managed?
- How are application signatures and provenance represented?
- How are reproducible builds and deployment artifacts supported?
- How are development and production application layouts related?
- How are locally developed applications installed and updated by a host?

The packaging model should preserve the distinction between an application's logical identity, its source code, its versions, and its running instances.

### 9.7 Application Interfaces and Presentation

Questions to resolve:
- How are application interfaces declared and discovered?
- How are WebView and browser interfaces hosted?
- How are CLI interfaces exposed?
- How do interfaces communicate with application instances?
- How are user identity and authentication handled?
- How are interfaces prevented from inheriting backend capabilities?
- How are interfaces exposed locally and remotely?
- How should interface hosting differ between desktop and server hosts?

The interface model should remain independent of application execution and capability authorization.

### 9.8 Provider Ecosystem and Extensibility

Questions to resolve:
- How are third-party capability providers implemented and registered?
- Can applications expose capabilities to other applications?
- How are provider dependencies and authority managed?
- How are provider versions and compatibility handled?
- Can providers be independently deployed or remotely hosted?
- How are providers inspected, trusted, and updated?

The initial implementation assumes trusted host-managed providers. A broader provider ecosystem will require additional security and lifecycle design.

### 9.9 Persistence, Synchronization, and State Management

Questions to resolve:
- How is application state stored and scoped?
- Which persistence mechanisms are built into the platform?
- How are databases and files provisioned?
- How are backups, migrations, and recovery handled?
- How are state and configuration shared across application instances?
- How are local and remote storage implementations presented through common contracts?
- What does synchronization mean for application data, configuration, and application versions?
- How are conflicts and offline changes handled?
- Which data belongs to an application, a user, a host, or a shared collaboration?

The architecture should preserve the separation between application state, host configuration, secrets, and capability authorization.

### 9.10 Resource Management and Scheduling

Questions to resolve:
- How are CPU and memory limits defined and enforced?
- How are long-running applications scheduled?
- How are background jobs and recurring tasks represented?
- How are application health and availability monitored?
- How does the platform scale application instances?
- Which responsibilities belong to the platform versus an external orchestrator?

Scheduling and orchestration should remain optional platform extensions rather than prerequisites for basic application execution.

### 9.11 Identity, Sharing, and Collaborative Use

Moving from a single-user environment to shared applications introduces new identity and authorization questions.

Questions to resolve:
- How are users, hosts, applications, and application instances identified?
- Who owns an application and its persistent data?
- How are applications shared with other users?
- Are grants made by an application owner, host administrator, resource owner, or some combination?
- How are user identity and permissions propagated across remote capability calls?
- How are revocation and ownership changes handled?
- How are audit records made understandable to users and administrators?

These questions can remain deferred during the initial single-user stages, but should be addressed before collaborative use is treated as a supported deployment model.

### 9.12 Development Experience and Tooling

The platform's developer experience is a core part of its value, particularly for independently developed and AI-generated applications.

Questions to resolve:
- How are applications developed and tested locally?
- What CLI tooling is needed?
- How are application manifests validated?
- How are TypeScript types generated or distributed?
- How are runtime errors, logs, and debugging information exposed?
- How do AI coding agents inspect and interact with the platform?
- How can an application be tested against different host environments?
- How are capability requirements and interface dependencies communicated to agents?
- How can an agent run an application, inspect failures, and iterate without receiving unnecessary host authority?

The developer experience should make the capability and composition models understandable and easy to use without weakening the underlying trust boundaries.

### 9.13 Decision-Making Principle

Future decisions should preserve the central architectural commitments established by this proposal:
- Applications remain independent of their execution environment.
- Authority is explicit and enforced by trusted components.
- Capability contracts are independent of their implementations and transports.
- Applications compose through explicit, typed, authorized connections.
- Application interfaces do not inherit backend privileges.
- The runtime remains reusable independently of the complete platform.
- Hosts retain control over policy, configuration, orchestration, and user experience.
- The platform remains useful without requiring a particular desktop interface or orchestration system.
- Security claims remain proportional to the actual enforcement mechanisms and threat model.

The goal is not to decide every implementation detail in advance. It is to establish stable boundaries that allow the platform to evolve without undermining its fundamental model.

## 10. Development and Validation Approach

The project should be developed as an evolving platform whose architecture is tested through implementation and real applications. The proposal establishes intended boundaries and direction, but it should not be treated as a reason to implement every anticipated feature before the system is useful.

### 10.1 Validate Through Working Systems

Prefer small end-to-end demonstrations over isolated infrastructure work when practical. A runtime feature should be exercised by an application. A capability abstraction should be exercised by a provider and a consumer. An application connection should be demonstrated by two independently implemented applications. A host API should be exercised by an actual host.

Each milestone should answer a concrete question about the platform. Examples include:
- Can an application execute without ambient filesystem or network authority?
- Can a host grant a narrowly scoped capability and inspect that grant?
- Can one application expose a typed interface that another application can consume?
- Can the provider implementation change without requiring the consumer application to change?
- Can a desktop host manage applications without owning their runtime or application logic?
- Can the same application model be used by a server host?

### 10.2 Use Real Applications to Drive the Model

Independent applications are not merely demonstrations added after the platform is complete. They are a way to discover whether the platform's abstractions are useful.

Build applications with distinct purposes and connect them through the platform. Avoid making every application a module of one tightly coupled product. When application development requires awkward workarounds, duplicated integrations, or direct access to another application's internals, treat that as evidence to revisit the platform model.

At the same time, avoid generalizing every one-off need into a platform feature. Prefer abstractions that support multiple real applications or preserve an important architectural boundary.

### 10.3 Record Architectural Decisions and Experiments

Maintain concise engineering records as the system develops. These should capture:
- Important architectural decisions and the context in which they were made.
- Alternatives considered and why they were accepted or rejected.
- Prototype results and implementation experiments.
- Known limitations, security assumptions, and deferred work.
- Changes to the application, capability, and connection models prompted by real use.

Decision records should explain both the selected approach and the constraints that may cause it to be revisited. They should distinguish demonstrated behavior from intended future behavior.

### 10.4 Keep Scope and Claims Aligned

The project should distinguish between implemented functionality, architectural requirements, planned stages, and speculative future directions. Documentation and demonstrations should not imply that deferred capabilities already exist or that an early security-conscious design provides hardened isolation.

The platform should grow in response to validated use cases while preserving its core principles: explicit authority, composable applications, replaceable hosts, and reusable execution infrastructure.
