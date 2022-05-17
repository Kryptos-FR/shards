#include "mesh.hpp"
#include "context.hpp"
#include "math.hpp"
#include <cassert>
#include <magic_enum.hpp>
#include <spdlog/spdlog.h>

namespace gfx {

size_t MeshFormat::computeVertexSize() const {
  size_t vertexSize = 0;
  for (auto &attr : vertexAttributes) {
    size_t typeSize = getVertexAttributeTypeSize(attr.type);
    vertexSize += attr.numComponents * typeSize;
  }
  return vertexSize;
}

void Mesh::update(const MeshFormat &format, const void *inVertexData, size_t vertexDataLength, const void *inIndexData,
                  size_t indexDataLength) {
  this->format = format;

  size_t vertexSize = format.computeVertexSize();
  size_t indexSize = getIndexFormatSize(format.indexFormat);

  calculateElementCounts(vertexDataLength, indexDataLength, vertexSize, indexSize);

  // FIXME: wgpu-rs requires buffer writes to be aligned to 4 currently
  vertexData.resize(alignTo(vertexDataLength, 4));
  indexData.resize(alignTo(indexDataLength, 4));

  memcpy(vertexData.data(), inVertexData, vertexDataLength);
  memcpy(indexData.data(), inIndexData, indexDataLength);

  update();
}

void Mesh::update(const MeshFormat &format, std::vector<uint8_t> &&vertexData, std::vector<uint8_t> &&indexData) {
  this->format = format;

  size_t vertexSize = format.computeVertexSize();
  size_t indexSize = getIndexFormatSize(format.indexFormat);

  this->vertexData = std::move(vertexData);
  this->indexData = std::move(indexData);
  calculateElementCounts(this->vertexData.size(), this->indexData.size(), vertexSize, indexSize);

  // FIXME: wgpu-rs requires buffer writes to be aligned to 4 currently
  this->vertexData.resize(alignTo(this->vertexData.size(), 4));
  this->indexData.resize(alignTo(this->indexData.size(), 4));

  update();
}

void Mesh::calculateElementCounts(size_t vertexDataLength, size_t indexDataLength, size_t vertexSize, size_t indexSize) {
  numVertices = vertexDataLength / vertexSize;
  assert(numVertices * vertexSize == vertexDataLength);

  numIndices = indexDataLength / indexSize;
  assert(numIndices * indexSize == indexDataLength);
}

void Mesh::update() {
  // This causes the GPU data to be recreated the next time it is requested
  contextData.reset();
}

void Mesh::initContextData(Context &context, MeshContextData &contextData) {
  WGPUDevice device = context.wgpuDevice;
  assert(device);

  contextData.format = format;
  contextData.numIndices = numIndices;
  contextData.numVertices = numVertices;

  WGPUBufferDescriptor desc = {};
  desc.size = vertexData.size();
  desc.usage = WGPUBufferUsage_Vertex | WGPUBufferUsage_CopyDst;
  contextData.vertexBuffer = wgpuDeviceCreateBuffer(device, &desc);
  contextData.vertexBufferLength = desc.size;

  wgpuQueueWriteBuffer(context.wgpuQueue, contextData.vertexBuffer, 0, vertexData.data(), vertexData.size());

  if (indexData.size() > 0) {
    desc.size = indexData.size();
    desc.usage = WGPUBufferUsage_Index | WGPUBufferUsage_CopyDst;
    contextData.indexBuffer = wgpuDeviceCreateBuffer(device, &desc);
    contextData.indexBufferLength = desc.size;

    wgpuQueueWriteBuffer(context.wgpuQueue, contextData.indexBuffer, 0, indexData.data(), indexData.size());
  }
}

} // namespace gfx
