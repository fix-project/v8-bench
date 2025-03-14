#pragma once

#include "v8instance.hh"
#include <chrono>
#include <cstdio>
#include <random>
#include <readerwriterqueue/readerwriterqueue.h>
#include <thread>

template <typename Request> class RequestGenerator {
  std::vector<std::shared_ptr<moodycamel::ReaderWriterQueue<Request>>>
      request_queues_;
  size_t request_per_second_;

  // Generate a request and push to a randomly selected request queue
  void generate_one_request(){
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
  RequestGenerator( size_t request_per_second )
    : request_queues_(),
      request_per_second_(request_per_second) {}

  void set_request_queues(  
      std::vector<std::shared_ptr<moodycamel::ReaderWriterQueue<Request>>>
          &&request_queues ) {
    request_queues_ = std::move( request_queues );
  }

  void start(){
    this->thread_ = std::thread(std::bind(&RequestGenerator<Request>::run, this));
  }
;
  ~RequestGenerator() {
    should_exit_ = true;
    thread_.join();
  }
};

template <typename Request> class V8Runner {
  V8Env& env_;
  std::shared_ptr<moodycamel::ReaderWriterQueue<Request>> requests_;
  bool should_exit_{};
  int processed_{};

void run() {
  V8Instance instance( env_ );

  Request request;
  while (not should_exit_) {
    while (not should_exit_ and not requests_->try_dequeue(request))
      ;

    if (should_exit_) {
      return;
    }

    instance.invoke(instance.instantiate(), request.func(), request.args());
    processed_++;
  }
}

  std::thread thread_{};

public:
V8Runner(V8Env &env,
    std::shared_ptr<moodycamel::ReaderWriterQueue<Request>> requests)
    : env_( env ), 
      requests_(requests) {}

    void start() {
      this->thread_ = std::thread(std::bind(&V8Runner<Request>::run, this));
    }

  ~V8Runner() {
    should_exit_ = true;
    printf("Processed %d\n", processed_);
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
