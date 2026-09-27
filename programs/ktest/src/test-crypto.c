#include "ktest.h"
#include "assert.h"

#include <crypto.h>
#include <string.h>

static void hash_to_hex(const uint8_t hash[32], char out[65]) {
  static const char digits[] = "0123456789abcdef";
  for (int i = 0; i < 32; i++) {
    out[i * 2] = digits[(hash[i] >> 4) & 0xF];
    out[i * 2 + 1] = digits[hash[i] & 0xF];
  }
  out[64] = '\0';
}

int test_sha256_empty_string(void) {
  SHA256_CTX ctx;
  uint8_t hash[32];
  char hex[65];

  sha256_init(&ctx);
  sha256_update(&ctx, (const uint8_t *)"", 0);
  sha256_final(&ctx, hash);
  hash_to_hex(hash, hex);

  T_ASSERT_STREQ(
      hex, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
  return 0;
}
CREATE_TEST(test_sha256_empty_string)

int test_sha256_abc(void) {
  SHA256_CTX ctx;
  uint8_t hash[32];
  char hex[65];

  sha256_init(&ctx);
  sha256_update(&ctx, (const uint8_t *)"abc", 3);
  sha256_final(&ctx, hash);
  hash_to_hex(hash, hex);

  T_ASSERT_STREQ(
      hex, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
  return 0;
}
CREATE_TEST(test_sha256_abc)

int test_sha256_pangram_single_block(void) {
  const char *msg = "The quick brown fox jumps over the lazy dog";
  SHA256_CTX ctx;
  uint8_t hash[32];
  char hex[65];

  sha256_init(&ctx);
  sha256_update(&ctx, (const uint8_t *)msg, strlen(msg));
  sha256_final(&ctx, hash);
  hash_to_hex(hash, hex);

  T_ASSERT_STREQ(
      hex, "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592");
  return 0;
}
CREATE_TEST(test_sha256_pangram_single_block)

int test_sha256_two_block_message(void) {
  const char *msg =
      "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
  SHA256_CTX ctx;
  uint8_t hash[32];
  char hex[65];

  sha256_init(&ctx);
  sha256_update(&ctx, (const uint8_t *)msg, strlen(msg));
  sha256_final(&ctx, hash);
  hash_to_hex(hash, hex);

  T_ASSERT_STREQ(
      hex, "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
  return 0;
}
CREATE_TEST(test_sha256_two_block_message)

int test_sha256_single_byte_updates_match_one_shot(void) {
  SHA256_CTX ctx;
  uint8_t hash[32];
  char hex[65];
  uint8_t one_a = 'a';

  sha256_init(&ctx);
  for (int i = 0; i < 1000; i++) {
    sha256_update(&ctx, &one_a, 1);
  }
  sha256_final(&ctx, hash);
  hash_to_hex(hash, hex);

  T_ASSERT_STREQ(
      hex, "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3");
  return 0;
}
CREATE_TEST(test_sha256_single_byte_updates_match_one_shot)

int test_sha256_odd_sized_chunks_match_one_shot(void) {
  const char *msg =
      "Nullex Nullex Nullex Nullex Nullex Nullex Nullex Nullex Nullex "
      "Nullex Nullex Nullex Nullex Nullex Nullex Nullex Nullex Nullex "
      "Nullex Nullex ";
  T_ASSERT_EQ(strlen(msg), 140);

  SHA256_CTX ctx;
  uint8_t hash[32];
  char hex[65];
  const uint8_t *p = (const uint8_t *)msg;

  sha256_init(&ctx);
  sha256_update(&ctx, p, 7);
  sha256_update(&ctx, p + 7, 64);
  sha256_update(&ctx, p + 71, 1);
  sha256_update(&ctx, p + 72, 64);
  sha256_update(&ctx, p + 136, 4);
  sha256_final(&ctx, hash);
  hash_to_hex(hash, hex);

  T_ASSERT_STREQ(
      hex, "83cd5b7b5c859fb2c5dcbf32fd63694d45c7c3187eda0cbc6af57d6f196829d7");
  return 0;
}
CREATE_TEST(test_sha256_odd_sized_chunks_match_one_shot)

int test_sha256_reinit_clears_previous_state(void) {
  SHA256_CTX ctx;
  uint8_t hash[32];
  char hex[65];

  sha256_init(&ctx);
  sha256_update(&ctx, (const uint8_t *)"abc", 3);
  uint8_t discard[32];
  sha256_final(&ctx, discard);

  sha256_init(&ctx);
  sha256_update(&ctx, (const uint8_t *)"", 0);
  sha256_final(&ctx, hash);
  hash_to_hex(hash, hex);

  T_ASSERT_STREQ(
      hex, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
  return 0;
}
CREATE_TEST(test_sha256_reinit_clears_previous_state)