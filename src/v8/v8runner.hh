#pragma once

#include <chrono>
#include <cstdio>
#include <iostream>
#include <memory>
#include <random>
#include <readerwriterqueue/readerwriterqueue.h>
#include <thread>
#include <v8-wasm.h>

#include "runtime.hh"
#include "v8instance.hh"

template <typename Request> class RequestGenerator {
  std::vector<std::shared_ptr<moodycamel::ReaderWriterQueue<Request>>>
      request_queues_;
  size_t request_per_second_;

  // Generate a request and push to a randomly selected request queue
  void generate_one_request() {
    std::random_device rd;
    std::mt19937 gen(rd());
    std::uniform_int_distribution<> distrib(0, request_queues_.size() - 1);
    request_queues_.at(distrib(gen))->try_enqueue({});
  }

  void busy_wait(std::chrono::nanoseconds interval) {
    auto start = std::chrono::steady_clock::now();

    while (true) {
      auto end = std::chrono::steady_clock::now();
      if (end - start > interval) {
        return;
      }
    }
  }

  // Pace request generation by Poisson distribution
  void poisson_request() {
    std::random_device rd;
    std::mt19937 gen(rd());
    std::poisson_distribution<> d(request_per_second_);

    auto interval = std::chrono::nanoseconds((long)(1e9 / d(gen)));
    busy_wait(interval);
    generate_one_request();
  }

  bool should_exit_{};
  void run() {
    while (not should_exit_) {
      poisson_request();
    }
  }
  std::thread thread_{};

public:
  RequestGenerator(size_t request_per_second)
      : request_queues_(), request_per_second_(request_per_second) {}

  void set_request_queues(
      std::vector<std::shared_ptr<moodycamel::ReaderWriterQueue<Request>>>
          &&request_queues) {
    request_queues_ = std::move(request_queues);
  }

  void start() {
    this->thread_ =
        std::thread(std::bind(&RequestGenerator<Request>::run, this));
  };
  ~RequestGenerator() {
    should_exit_ = true;
    thread_.join();
  }
};

template <typename Request> class V8Runner {
  V8Env &env_;
  std::shared_ptr<moodycamel::ReaderWriterQueue<Request>> requests_;
  bool should_exit_{};
  int processed_{};
  std::vector<int> logs_{};

  void run() {
    V8Instance instance(env_);

    Request request;
    while (not should_exit_) {
      while (not should_exit_ and not requests_->try_dequeue(request))
        ;

      if (should_exit_) {
        return;
      }

      auto start = std::chrono::steady_clock::now();
      instance.invoke(instance.instantiate(), request.func(), request.args());
      auto end = std::chrono::steady_clock::now();
      logs_.push_back(
          std::chrono::duration_cast<std::chrono::nanoseconds>(end - start)
              .count());
      processed_++;
    }
  }

  std::thread thread_{};

public:
  V8Runner(V8Env &env,
           std::shared_ptr<moodycamel::ReaderWriterQueue<Request>> requests)
      : env_(env), requests_(requests) {}

  void start() {
    this->thread_ = std::thread(std::bind(&V8Runner<Request>::run, this));
  }

  ~V8Runner() {
    should_exit_ = true;
    printf("Processed %d\n", processed_);
    std::sort(logs_.begin(), logs_.end());
    printf("P99: %dns\n", logs_[0.99 * processed_]);
    printf("P90: %dns\n", logs_[0.9 * processed_]);
    printf("P50: %dns\n", logs_[0.5 * processed_]);
    thread_.join();
  }
};

template <typename Request> class V8Runtime {
  V8Env env_;
  RequestGenerator<Request> request_generator_;
  std::vector<std::unique_ptr<V8Runner<Request>>> runners_{};

public:
  V8Runtime(char *argv0, bool bounds_checks, std::span<uint8_t> wasm_bin,
            size_t number_of_threads, size_t request_per_second)
      : env_(argv0, bounds_checks), request_generator_(request_per_second) {
    env_.compile(wasm_bin);
    std::vector<std::shared_ptr<moodycamel::ReaderWriterQueue<Request>>>
        request_queues{};
    for (size_t i = 0; i < number_of_threads; i++) {
      auto q = std::make_shared<moodycamel::ReaderWriterQueue<Request>>();
      request_queues.push_back(q);
      runners_.emplace_back(std::make_unique<V8Runner<Request>>(env_, q));
    }

    request_generator_.set_request_queues(std::move(request_queues));
  }

  void start() {
    for (auto &r : runners_) {
      r->start();
    }
    request_generator_.start();
  }
};

template <typename Request> class V8DirectRunner {
  V8Env &env_;
  bool should_exit_{};
  std::atomic<int> processed_{};
  std::optional<v8::CompiledWasmModule> module_;
  bool new_isolate_per_call_{};

  void run() {
    std::unique_ptr<V8Instance> instance =
        std::make_unique<V8Instance>(env_, module_.value());

    Request request;
    while (not should_exit_) {
      {
        instance->invoke(instance->instantiate(), request.func(),
                         request.args());
      }
      processed_++;

      if (new_isolate_per_call_) {
        instance.reset();
        instance = std::make_unique<V8Instance>(env_, module_.value());
      }
    }
  }

  std::thread thread_{};

public:
  V8DirectRunner(V8Env &env, std::span<uint8_t> wasm_bin,
                 bool new_isolate_per_call)
      : env_(env), new_isolate_per_call_(new_isolate_per_call) {
    env_.compile(wasm_bin);
    module_.emplace(env.get_compiled_wasm());
  }

  void start() {
    this->thread_ = std::thread(std::bind(&V8DirectRunner::run, this));
  }

  int report() {
    should_exit_ = true;
    return processed_;
  }

  ~V8DirectRunner() {
    should_exit_ = true;
    thread_.join();
  }
};

template <typename Request> class V8DirectRuntime : public Runtime {
  V8Env env_;
  std::vector<std::unique_ptr<V8DirectRunner<Request>>> runners_{};

public:
  V8DirectRuntime(char *argv0, bool new_isolate_per_call,
                  std::span<uint8_t> wasm_bin, size_t number_of_threads)
      : env_(argv0) {
    for (size_t i = 0; i < number_of_threads; i++) {
      runners_.emplace_back(std::make_unique<V8DirectRunner<Request>>(
          env_, wasm_bin, new_isolate_per_call));
    }
  }

  void start() override {
    for (auto &r : runners_) {
      r->start();
    }
  }

  int report() override {
    int result = 0;
    for (auto &r : runners_) {
      result += r->report();
    }
    return result;
  }
};
