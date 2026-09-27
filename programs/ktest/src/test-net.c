#include "ktest.h"
#include "assert.h"

#include <nullex/net.h>
#include <stdint.h>

int test_csocket_returns_valid_descriptor(void) {
  int32_t fd = csocket();
  T_ASSERT(fd >= 0);
  closesock(fd);
  return 0;
}
CREATE_TEST(test_csocket_returns_valid_descriptor)

int test_connsock_to_unresolvable_host_fails(void) {
  int32_t fd = csocket();
  T_ASSERT(fd >= 0);

  int32_t ret = connsock(fd, "this.host.should.not.resolve.invalid", 80);
  T_ASSERT(ret < 0);

  closesock(fd);
  return 0;
}
CREATE_TEST(test_connsock_to_unresolvable_host_fails)

int test_send_on_unconnected_socket_fails(void) {
  int32_t fd = csocket();
  T_ASSERT(fd >= 0);

  const uint8_t data[] = {'h', 'i'};
  int32_t ret = send(fd, data, sizeof(data));
  T_ASSERT(ret < 0);

  closesock(fd);
  return 0;
}
CREATE_TEST(test_send_on_unconnected_socket_fails)

int test_recv_on_unconnected_socket_fails(void) {
  int32_t fd = csocket();
  T_ASSERT(fd >= 0);

  uint8_t buf[64];
  struct nlx_msghdr msg;
  msg.type = NLX_IO_RAW;
  msg._padding = 0;
  msg.payload.raw.buf = buf;
  msg.payload.raw.len = sizeof(buf);

  int32_t ret = recv(fd, &msg);
  T_ASSERT(ret < 0);

  closesock(fd);
  return 0;
}
CREATE_TEST(test_recv_on_unconnected_socket_fails)

int test_closesock_is_safe_on_fresh_and_reused_fd(void) {
  int32_t fd1 = csocket();
  T_ASSERT(fd1 >= 0);
  closesock(fd1);

  int32_t fd2 = csocket();
  T_ASSERT(fd2 >= 0);
  closesock(fd2);

  return 0;
}
CREATE_TEST(test_closesock_is_safe_on_fresh_and_reused_fd)

#if 0
int test_connsock_and_send_recv_round_trip(void) {
  int32_t fd = csocket();
  T_ASSERT(fd >= 0);

  int32_t ret = connsock(fd, "example.com", 80);
  T_ASSERT(ret >= 0);

  const char *request = "GET / HTTP/1.0\r\nHost: example.com\r\n\r\n";
  int32_t sent = send(fd, (const uint8_t *)request, strlen(request));
  T_ASSERT(sent >= 0);

  uint8_t buf[256];
  struct nlx_msghdr msg;
  msg.type = NLX_IO_RAW;
  msg._padding = 0;
  msg.payload.raw.buf = buf;
  msg.payload.raw.len = sizeof(buf);

  int32_t received = recv(fd, &msg);
  T_ASSERT(received > 0);

  closesock(fd);
  return 0;
}
CREATE_TEST(test_connsock_and_send_recv_round_trip)
#endif