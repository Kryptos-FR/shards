/* SPDX-License-Identifier: BSD 3-Clause "New" or "Revised" License */
/* Copyright © 2019 Giovanni Petrantoni */

#ifndef CB_BLOCKSCHAIN_HPP
#define CB_BLOCKSCHAIN_HPP

#include "chainblocks.h"
#include <string>
#include <vector>

namespace chainblocks {
class CBException : public std::exception {
public:
  explicit CBException(const char *errmsg) : errorMessage(errmsg) {}
  explicit CBException(std::string errmsg) : errorMessage(errmsg) {}

  [[nodiscard]] const char *what() const noexcept override {
    return errorMessage.c_str();
  }

private:
  std::string errorMessage;
};

CBlock *createBlock(const char *name);

struct Var : public CBVar {
  explicit Var() : CBVar() {
    valueType = None;
    payload.chainState = CBChainState::Continue;
  }

  explicit Var(const CBVar &other) {
    memcpy((void *)this, (void *)&other, sizeof(CBVar));
  }

  explicit operator bool() const {
    if (valueType != Bool) {
      throw CBException("Invalid variable casting! expected Bool");
    }
    return payload.boolValue;
  }

  explicit operator int() const {
    if (valueType != Int) {
      throw CBException("Invalid variable casting! expected Int");
    }
    return static_cast<int>(payload.intValue);
  }

  explicit operator uintptr_t() const {
    if (valueType != Int) {
      throw CBException("Invalid variable casting! expected Int");
    }
    return static_cast<uintptr_t>(payload.intValue);
  }

  explicit operator int16_t() const {
    if (valueType != Int) {
      throw CBException("Invalid variable casting! expected Int");
    }
    return static_cast<int16_t>(payload.intValue);
  }

  explicit operator uint8_t() const {
    if (valueType != Int) {
      throw CBException("Invalid variable casting! expected Int");
    }
    return static_cast<uint8_t>(payload.intValue);
  }

  explicit operator int64_t() const {
    if (valueType != Int) {
      throw CBException("Invalid variable casting! expected Int");
    }
    return payload.intValue;
  }

  explicit operator float() const {
    if (valueType == Float) {
      return static_cast<float>(payload.floatValue);
    } else if (valueType == Int) {
      return static_cast<float>(payload.intValue);
    } else {
      throw CBException("Invalid variable casting! expected Float");
    }
  }

  explicit operator double() const {
    if (valueType == Float) {
      return payload.floatValue;
    } else if (valueType == Int) {
      return static_cast<double>(payload.intValue);
    } else {
      throw CBException("Invalid variable casting! expected Float");
    }
  }

  static Var Stop() {
    Var res;
    res.valueType = None;
    res.payload.chainState = CBChainState::Stop;
    return res;
  }

  static Var Restart() {
    Var res;
    res.valueType = None;
    res.payload.chainState = CBChainState::Restart;
    return res;
  }

  static Var Return() {
    Var res;
    res.valueType = None;
    res.payload.chainState = CBChainState::Return;
    return res;
  }

  static Var Rebase() {
    Var res;
    res.valueType = None;
    res.payload.chainState = CBChainState::Rebase;
    return res;
  }

  template <typename T>
  static Var Object(T valuePtr, uint32_t objectVendorId,
                    uint32_t objectTypeId) {
    Var res;
    res.valueType = CBType::Object;
    res.payload.objectValue = valuePtr;
    res.payload.objectVendorId = objectVendorId;
    res.payload.objectTypeId = objectTypeId;
    return res;
  }

  template <typename T>
  static Var Enum(T value, uint32_t enumVendorId, uint32_t enumTypeId) {
    Var res;
    res.valueType = CBType::Enum;
    res.payload.enumValue = CBEnum(value);
    res.payload.enumVendorId = enumVendorId;
    res.payload.enumTypeId = enumTypeId;
    return res;
  }

  Var(uint8_t *ptr, int64_t size) : CBVar() {
    valueType = Bytes;
    payload.bytesSize = size;
    payload.bytesValue = ptr;
  }

  explicit Var(int src) : CBVar() {
    valueType = Int;
    payload.intValue = src;
  }

  explicit Var(int a, int b) : CBVar() {
    valueType = Int2;
    payload.int2Value[0] = a;
    payload.int2Value[1] = b;
  }

  explicit Var(int64_t a, int64_t b) : CBVar() {
    valueType = Int2;
    payload.int2Value[0] = a;
    payload.int2Value[1] = b;
  }

  explicit Var(double a, double b) : CBVar() {
    valueType = Float2;
    payload.float2Value[0] = a;
    payload.float2Value[1] = b;
  }

  explicit Var(float a, float b) : CBVar() {
    valueType = Float2;
    payload.float2Value[0] = a;
    payload.float2Value[1] = b;
  }

  explicit Var(double src) : CBVar() {
    valueType = Float;
    payload.floatValue = src;
  }

  explicit Var(bool src) : CBVar() {
    valueType = Bool;
    payload.boolValue = src;
  }

  explicit Var(CBSeq seq) : CBVar() {
    valueType = Seq;
    payload.seqValue = seq;
  }

  explicit Var(CBChain *src) : CBVar() {
    valueType = Chain;
    payload.chainValue = src;
  }

  explicit Var(CBImage img) : CBVar() {
    valueType = Image;
    payload.imageValue = img;
  }

  explicit Var(uint64_t src) : CBVar() {
    valueType = Int;
    payload.intValue = src;
  }

  explicit Var(int64_t src) : CBVar() {
    valueType = Int;
    payload.intValue = src;
  }

  explicit Var(const char *src) : CBVar() {
    valueType = CBType::String;
    payload.stringValue = src;
  }

  explicit Var(const std::string &src) : CBVar() {
    valueType = CBType::String;
    payload.stringValue = src.c_str();
  }

  explicit Var(CBTable &src) : CBVar() {
    valueType = Table;
    payload.tableValue = src;
  }

  explicit Var(CBColor color) : CBVar() {
    valueType = Color;
    payload.colorValue = color;
  }

  explicit Var(CBSeq &storage, const std::vector<std::string> &strings)
      : CBVar() {
    valueType = Seq;
    for (auto &str : strings) {
      stbds_arrpush(storage, Var(str));
    }
    payload.seqValue = storage;
  }
};

class ChainProvider {
  // used specially for live editing chains, from host languages
public:
  class Info {
  public:
    Info() {
      _info.objectVendorId = 'frag';
      _info.objectTypeId = 'chnp';

      CBTypeInfo none{};
      stbds_arrpush(_providerOrNone, none);
      stbds_arrpush(_providerOrNone, _info);
    }

    ~Info() { stbds_arrfree(_providerOrNone); }

    operator CBTypeInfo() const { return _info; }

    operator CBTypesInfo() const { return _providerOrNone; }

  private:
    CBTypeInfo _info{CBType::Object};
    CBTypesInfo _providerOrNone = nullptr;
  };

  struct Update {
    const char *error;
    CBChain *chain;
  };

  static inline Info Info{};

  ChainProvider() {}
  ~ChainProvider() {}

  virtual void reset() = 0;

  virtual bool ready() = 0;
  virtual void setup(const char *path, const CBTypeInfo &inputType,
                     const CBExposedTypesInfo &consumables) = 0;

  virtual bool updated() = 0;
  virtual Update acquire() = 0;
  virtual void release(CBChain *chain) = 0;
  virtual void release(const char *error) = 0;
};

class Chain {
public:
  template <typename String>
  Chain(String name) : _name(name), _looped(false), _unsafe(false) {}
  Chain() : _looped(false), _unsafe(false) {}

  template <typename String, typename... Vars>
  Chain &block(String name, Vars... params) {
    std::string blockName(name);
    auto blk = createBlock(blockName.c_str());
    blk->setup(blk);

    std::vector<Var> vars = {Var(params)...};
    for (size_t i = 0; i < vars.size(); i++) {
      blk->setParam(blk, i, vars[i]);
    }

    _blocks.push_back(blk);
    return *this;
  }

  template <typename V> Chain &let(V value) {
    auto blk = createBlock("Const");
    blk->setup(blk);
    blk->setParam(blk, 0, Var(value));
    _blocks.push_back(blk);
    return *this;
  }

  Chain &looped(bool looped) {
    _looped = looped;
    return *this;
  }

  Chain &unsafe(bool unsafe) {
    _unsafe = unsafe;
    return *this;
  }

  template <typename String> Chain &name(String name) {
    _name = name;
    return *this;
  }

  operator CBChain *();

  operator CBVar() {
    CBVar res{};
    res.valueType = Seq;
    for (auto blk : _blocks) {
      CBVar blkVar{};
      blkVar.valueType = Block;
      blkVar.payload.blockValue = blk;
      stbds_arrpush(res.payload.seqValue, blkVar);
    }
    // blocks are unique so drain them here
    _blocks.clear();
    return res;
  }

private:
  std::string _name;
  bool _looped;
  bool _unsafe;
  std::vector<CBlock *> _blocks;
};
} // namespace chainblocks

#endif
