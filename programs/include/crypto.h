/**
 * @file crypto.h
 * @brief SHA-256 cryptographic hashing implementation interface. (and more to
 * be added)
 *
 * Provides the SHA-256 hashing context and functions required to initialise,
 * update, and finalise a SHA-256 hash calculation.
 */

#include <stddef.h>
#include <stdint.h>

/**
 * @brief SHA-256 round constants.
 *
 * Constant values specified by the SHA-256 algorithm and used during each
 * of the 64 compression rounds.
 */
static const uint32_t k[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
    0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
    0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
    0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
    0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
    0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2};

/**
 * @brief Hexadecimal characters used for hash representation.
 */
static const char hex_chars[] = "0123456789abcdef";

/**
 * @brief Rotate a 32-bit value to the right.
 *
 * @param[in] x Value to rotate.
 * @param[in] n Number of bit positions to rotate.
 */
#define ROTR(x, n) (((x) >> (n)) | ((x) << (32 - (n))))

/**
 * @brief SHA-256 choose function.
 *
 * Selects bits from @p y or @p z based on the corresponding bits of @p x.
 *
 * @param[in] x First input value.
 * @param[in] y Second input value.
 * @param[in] z Third input value.
 */
#define CH(x, y, z) (((x) & (y)) ^ ((~(x)) & (z)))

/**
 * @brief SHA-256 majority function.
 *
 * Determines the majority value of each corresponding bit in the three
 * input words.
 *
 * @param[in] x First input value.
 * @param[in] y Second input value.
 * @param[in] z Third input value.
 */
#define MAJ(x, y, z) (((x) & (y)) ^ ((x) & (z)) ^ ((y) & (z)))

/**
 * @brief SHA-256 uppercase Sigma 0 function.
 *
 * @param[in] x Input 32-bit word.
 */
#define EP0(x) (ROTR(x, 2) ^ ROTR(x, 13) ^ ROTR(x, 22))

/**
 * @brief SHA-256 uppercase Sigma 1 function.
 *
 * @param[in] x Input 32-bit word.
 */
#define EP1(x) (ROTR(x, 6) ^ ROTR(x, 11) ^ ROTR(x, 25))

/**
 * @brief SHA-256 lowercase sigma 0 function.
 *
 * @param[in] x Input 32-bit word.
 */
#define SIG0(x) (ROTR(x, 7) ^ ROTR(x, 18) ^ ((x) >> 3))

/**
 * @brief SHA-256 lowercase sigma 1 function.
 *
 * @param[in] x Input 32-bit word.
 */
#define SIG1(x) (ROTR(x, 17) ^ ROTR(x, 19) ^ ((x) >> 10))

/**
 * @brief SHA-256 hashing context.
 *
 * Stores the intermediate state of a SHA-256 hashing operation, allowing
 * data to be processed incrementally across multiple calls to
 * @c sha256_update().
 */
typedef struct {
  /**
   * @brief Internal 512-bit message block buffer.
   *
   * Stores input data until enough bytes are available to process a complete
   * SHA-256 block.
   */
  uint8_t data[64];

  /**
   * @brief Number of bytes currently stored in @c data.
   */
  uint32_t datalen;

  /**
   * @brief Total number of processed input bits.
   */
  uint64_t bitlen;

  /**
   * @brief Current SHA-256 intermediate hash state.
   *
   * Contains the eight 32-bit words maintained by the SHA-256 algorithm.
   */
  uint32_t state[8];
} SHA256_CTX;

/**
 * @brief Process a single 512-bit SHA-256 message block.
 *
 * Performs the SHA-256 compression function on one complete 64-byte block
 * and updates the intermediate hash state stored in the context.
 *
 * @param[in,out] ctx SHA-256 context whose state is updated.
 * @param[in] data 64-byte message block to process.
 */
static void sha256_transform(SHA256_CTX *ctx, const uint8_t data[]);

/**
 * @brief Initialise a SHA-256 hashing context.
 *
 * Resets the supplied context and initialises it with the standard SHA-256
 * initial hash values.
 *
 * @param[out] ctx SHA-256 context to initialise.
 */
void sha256_init(SHA256_CTX *ctx);

/**
 * @brief Add data to an ongoing SHA-256 hash calculation.
 *
 * The supplied data may be provided in multiple calls. The SHA-256 context
 * maintains any incomplete message block until sufficient data is available
 * for processing.
 *
 * @param[in,out] ctx SHA-256 context to update.
 * @param[in] data Input data to incorporate into the hash.
 * @param[in] len Number of bytes in @p data.
 */
void sha256_update(SHA256_CTX *ctx, const uint8_t data[], size_t len);

/**
 * @brief Finalise a SHA-256 hash calculation.
 *
 * Applies SHA-256 padding, processes the final message block, and writes the
 * resulting 256-bit digest to the supplied output buffer.
 *
 * @param[in,out] ctx SHA-256 context to finalise.
 * @param[out] hash Buffer receiving the 32-byte SHA-256 digest.
 *
 * @note The @p hash buffer must contain space for at least 32 bytes.
 */
void sha256_final(SHA256_CTX *ctx, uint8_t hash[]);