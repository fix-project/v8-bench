#pragma once

class Runtime {
public:
  virtual void start() = 0;
  virtual int report() = 0;
  virtual ~Runtime(){};
};
