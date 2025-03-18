#pragma once

#include <functional>
#include <thread>

template <typename Request, typename W2Cmodule> class W2CDirectRunner {
  bool should_exit_{};
  std::atomic<int> processed_{};

  void run() {
    Request request;
    while (not should_exit_) {
      W2Cmodule instance;
      instance.invoke(request.args());
      processed_++;
    }
  }

  std::thread thread_{};

public:
  W2CDirectRunner(){};

  void start() {
    this->thread_ = std::thread(std::bind(&W2CDirectRunner::run, this));
  }

  int report() {
    should_exit_ = true;
    return processed_;
  }

  ~W2CDirectRunner() {
    should_exit_ = true;
    thread_.join();
  }
};

template <typename Request, typename W2Cmodule> class W2CDirectRuntime {
  std::vector<std::unique_ptr<W2CDirectRunner<Request, W2Cmodule>>> runners_{};

public:
  W2CDirectRuntime(size_t number_of_threads) {
    for (size_t i = 0; i < number_of_threads; i++) {
      runners_.emplace_back(
          std::make_unique<W2CDirectRunner<Request, W2Cmodule>>());
    }
  }

  void start() {
    for (auto &r : runners_) {
      r->start();
    }
  }

  int report() {
    int result = 0;
    for (auto &r : runners_) {
      result += r->report();
    }
    return result;
  }
};
