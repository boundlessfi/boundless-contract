# Testnet grant run, events 2.1.0

Run 2026-10-08T04:37:15.647Z against `CBEODVJGUYCIYTVXD7KI5UG3BJ2UE4T7AGI2TGY3T4Q5GQRFGTRYVTZP` with the BGT test asset `CBEOUEMDTM56NRTOOSXCPGMD33RPD5YWEKJ542HAEVREAYCC4QIZ3XIU`.
337 of 337 checks passed across 163 transactions.
Highest fee paid by one transaction: 10569590 stroops.

## setup

- pass: BGT CBEOUEMDTM56NRTOOSXCPGMD33RPD5YWEKJ542HAEVREAYCC4QIZ3XIU is whitelisted on CBEODVJGUYCIYTVXD7KI5UG3BJ2UE4T7AGI2TGY3T4Q5GQRFGTRYVTZP (2.1.0)

  - [provision 2 accounts](https://stellar.expert/explorer/testnet/tx/9298e9835ae5a493f0aab66f2a210acc0551b5f032993d21595aa02ddae8c85f)
  - [provision 4 accounts](https://stellar.expert/explorer/testnet/tx/71f593423785cf5d9138b33d7dc7d8e77214b516194a54967deaffca6f0978cc)
  - [provision 15 accounts](https://stellar.expert/explorer/testnet/tx/85b76ddf167c4a1d60ce197293e92961da2b928cc6bdbca5fb118f6f942e53bd)
  - [provision 15 accounts](https://stellar.expert/explorer/testnet/tx/c812a8e1bf74300bf449d41a074418107f0356d6bcbc3ab6aef3c2e833e2d432)
  - [provision 15 accounts](https://stellar.expert/explorer/testnet/tx/bf12a4961f0abdc62f60ceb0e080ef61032fb46eb39b436a9fae64070f3f31cd)
  - [provision 7 accounts](https://stellar.expert/explorer/testnet/tx/87101d1c623c0845d20127838795b27de19eabd767f46ea895f188e2109fcd71)
  - [provision 3 accounts](https://stellar.expert/explorer/testnet/tx/637658e5fa741ba6bb6a478ce56d18f3b3809e35d0a7eecc1714cb90c44d9c8e)

## micro grant

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: select three recipients
- pass: release milestone 0 to GBYPFH
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GB5H52
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBT7XO
- pass: milestone 0 paid exactly
- pass: completes when the last award is paid

  - [create_event](https://stellar.expert/explorer/testnet/tx/c7fa85248d94d07c056c9bd55124821708f4adadcb07669b6d29d15bd9d90994)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/10ff28736c6415ad7c6a5839823a8dd91d2f575ffbf365406352c23a74d6e1bd)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/cc496f52831dc6740c91426fcb43d1592a92afd62c4d00f15ddb6102ef687e10)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/8f4b54167dee3041505b5a77eea08d1d25227b2f5bf687afbe0e31cdb7e57fec)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/d71e3647f9fa42e8cd64d74472be96db5b42f5ac509ad88d27546b154d75a1d0)

## tiered, rounding, any order

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: select three tiers
- pass: release milestone 2 to GC6YTP
- pass: milestone 2 paid exactly
- pass: release milestone 0 to GB72DO
- pass: milestone 0 paid exactly
- pass: release milestone 1 to GD6H3Z
- pass: milestone 1 paid exactly
- pass: release milestone 2 to GB72DO
- pass: milestone 2 paid exactly
- pass: release milestone 0 to GC6YTP
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GD6H3Z
- pass: milestone 0 paid exactly
- pass: release milestone 1 to GB72DO
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GC6YTP
- pass: milestone 1 paid exactly
- pass: release milestone 2 to GD6H3Z
- pass: milestone 2 paid exactly
- pass: tier 1 received its whole award
- pass: tier 2 received its whole award
- pass: completed

  - [create_event](https://stellar.expert/explorer/testnet/tx/1bd70598369abe4b724d57eb5c38d4c016d083c3ef9dc83178831cf4eefdeacb)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/5107ed6edc330d3135c1fc84be0ae92b780c36abc09dbc833f520f825bdcb093)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/696fc666ef35d03ce0889dc4272911f1694cff03951105856af10bcca3f8f735)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/8d0729307fe2cfe9469a82534fbbdb06a8e5b398ae970f0d26e8eac4f8143bee)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/cd41ecc27bf284f8142ab61a97bf7c992db25791725eec0c6853e7db97d0d2c6)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/05512a32bf0db5931af7498258b3e14c517014ac6d3de51557a407c19f9d9459)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/1682e202d5de4f515ac001191d9e265026aad653b1c73e1f18ac9f1b706ccc99)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/78b159a5fd65856f0d143240c07b31f4cfa1904bf5cf47cf72e0460a449f38ac)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/16082e1ece2f103ff76228da56322550dfc62bb5f87f6919dfac37b67a22282a)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/7c77f3c1d8b9c518130e04e301aaefb9acbb83abef97f90968ff9d6b49a7ee95)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/30ccba82d0b02a2ac9eb99bf9e08c6bddda8d201f36845864da1936f23e70576)

## milestone split, any order

- pass: a split that does not add up to the whole award
- pass: publish the grant
- pass: owner paid budget plus fee
- pass: the split is stored on the grant
- pass: select two awards of different sizes
- pass: release milestone 2 to GDGKLG
- pass: milestone 2 paid exactly
- pass: release milestone 0 to GAFPD2
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDGKLG
- pass: milestone 0 paid exactly
- pass: release milestone 1 to GAFPD2
- pass: milestone 1 paid exactly
- pass: release milestone 2 to GAFPD2
- pass: milestone 2 paid exactly
- pass: release milestone 1 to GDGKLG
- pass: milestone 1 paid exactly
- pass: the larger award was paid exactly
- pass: the smaller award was paid exactly
- pass: completes when the last share is paid

  - [provision 2 accounts](https://stellar.expert/explorer/testnet/tx/e3a7dd09d1ccccb596ecf37fc8b278f37d88f204dba66b111a3b05ec754fb102)
  - [create_event](https://stellar.expert/explorer/testnet/tx/2988397b493a06c7fc2b5599d88b2f88db4be0d56eb11b4f796343d10ae03b09)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/641cebbd867cfa01df7b0d046445563ee5931eb9c165acb4b2c47526333073dd)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/692df50d77694169c00aaf0e28e5cca27ca58900ae89f73f6e05d57113963ac2)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/bcdba5746da342db99316c38fe1e99e0537f7cfba6843d827e1fc5b1afe3375a)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/7aaa5a560ef83054e80906b62c2c5401305d48dfd58712c0a244a61ef1a3770b)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/ea1db9034654679388184af27f4f28b65cc5d2058b00f66d2765b93aebb0e54d)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/39aa7104f09888f32908bcd409eb40931eda321254947822cdcf71918e6b1d64)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/21e3f0982a198def0d16a5a986873f9d6d9256766e3402c02a439fc37e2d66be)

## forfeit and the cancel guard

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: select one recipient
- pass: release milestone 0 to GAYYPP
- pass: milestone 0 paid exactly
- pass: forfeit milestone 1
- pass: a forfeit pays nothing
- pass: cancel refused while an award is owed (AwardsOutstanding)
- pass: release milestone 2 to GAYYPP
- pass: milestone 2 paid exactly
- pass: start cancel
- pass: grant cancelled
- pass: forfeited share returned to the owner

  - [create_event](https://stellar.expert/explorer/testnet/tx/0484888898b6e7e791d18c1d663d231bbcc93d577b8dbb75c23b773bdac72dec)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/1e6f237d19aa3e807888082f858399739b09bccb07d67a52903a461ceb9388ce)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/4f4d4b64eded01895535b40772ba757bde584c829c3c4c575ef624b14c9ba8d9)
  - [forfeit_milestone](https://stellar.expert/explorer/testnet/tx/3992159ddef33536b3127cb7fd2d598398263362d8bdf7398c7b19849edfbfad)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/6af71c0f3dd23ede021f319a412f5d6713586e56722c22e1554b8ab835f912b9)
  - [start_cancel](https://stellar.expert/explorer/testnet/tx/1d0d3d7bf16f7254aff13a80465a3d99bc1115b57d82fb00a2f8d04c95b26ae5)

## partners, pro rata

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: partner adds 20.0000000
- pass: partner paid amount plus fee
- pass: award more than the budget, funded by the partner
- pass: release milestone 0 to GBQ6R5
- pass: milestone 0 paid exactly
- pass: partner adds 15.0000000
- pass: partner paid amount plus fee
- pass: release milestone 1 to GBQ6R5
- pass: milestone 1 paid exactly
- pass: the late top-up keeps the grant open
- pass: start cancel
- pass: an empty refund batch is refused (InvalidBatchSize)
- pass: anyone can crank
- pass: anyone can finalize
- pass: partner 1 refunded pro rata
- pass: partner 2 refunded pro rata
- pass: owner gets nothing when partners are short

  - [create_event](https://stellar.expert/explorer/testnet/tx/233b0e061a89baa4d7cfd9be9665f6a5cf77c9ff1168f4d40c7bb6269515fb06)
  - [add_funds](https://stellar.expert/explorer/testnet/tx/8ad5aa4f2ae13f25f5387a7984e2c57821ebee646238c98f81419f4f279ee2af)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/04131c1b121dc4a7331ed637013430da9b5f743d742b5e1385e5e06690e82f95)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/e398e24f7016b22e32ef390d1cfe011b8b671d93d55886d9f1912a0c48f03e67)
  - [add_funds](https://stellar.expert/explorer/testnet/tx/dbadb3c1020e11fa49083033c4423d677dbbe51cf64bc0b1a76ef863632b832e)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/79388634bd48fdfae8dea9b605986c98c120e2753b424804908c8234486f9f0a)
  - [start_cancel](https://stellar.expert/explorer/testnet/tx/bfe19444fcd5d413e7255794f9a27f624bb1bb341daad66bae7011cbde5b2749)
  - [process_cancel_batch](https://stellar.expert/explorer/testnet/tx/a567132975aaa6f6afaa73621394e57d39932681b5200c4781b317c6ef2a8d20)
  - [finalize_cancel](https://stellar.expert/explorer/testnet/tx/e434e2df69fbbeec55281cf05a8a4f82151b912a5deff8223219e26e4b503281)

## frozen partner

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: partner adds 10.0000000
- pass: partner paid amount plus fee
- pass: partner adds 10.0000000
- pass: partner paid amount plus fee
- pass: start cancel
- pass: refund batch
- pass: finalize cancel
- pass: grant cancelled
- pass: other partners are refunded
- pass: owner residual paid
- pass: frozen partner refund held
- pass: a frozen account cannot take its refund yet (HostError: Error(Contract, #11))
- pass: refund claimed after unfreezing
- pass: frozen partner made whole
- pass: a second claim is refused (NoRefundOwed)

  - [create_event](https://stellar.expert/explorer/testnet/tx/6f0957d5f2ad594562e5a417d48f7e10ff49333ca94955e3eb974ef78219dc9f)
  - [add_funds](https://stellar.expert/explorer/testnet/tx/ceb403cf8fc340f6ccf077d01f3841f8b014d656ebc7aa12457d2629c6a5b394)
  - [add_funds](https://stellar.expert/explorer/testnet/tx/8cc13e703a33564bd850cb0840d488ed0dd5196897771ee48fcb0aad908d621f)
  - [freeze](https://stellar.expert/explorer/testnet/tx/ac5521bdca4f3b78b3b2d111524c907b5cec150ebd4b08774e8a288f214e3b58)
  - [start_cancel](https://stellar.expert/explorer/testnet/tx/21030978f54baaf5482d50a0fc16f7bbc9a8e577e9ebe78ae55d924e3b08eafd)
  - [process_cancel_batch](https://stellar.expert/explorer/testnet/tx/55a9086545ca7214261c3d61dd58658054d6f6ff27fe37d902d2c77b522233f4)
  - [finalize_cancel](https://stellar.expert/explorer/testnet/tx/7ce9009150a3a8e9cc2f4b973613432ae88ce0350f40f3092b8525b1630accdd)
  - [unfreeze](https://stellar.expert/explorer/testnet/tx/9b5618debaf3a5f1249428dc920d3576b70273282e4f0b15d595cfad0f9fd5ce)
  - [claim_refund](https://stellar.expert/explorer/testnet/tx/2e8b543c562beaf16204110611ed0f1cf4dbec64a991f8e5299a5a5face3ef89)

## refusals

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: one recipient, two awards (DuplicateRecipient)
- pass: award smaller than its milestones
- pass: reputation bump over the cap
- pass: more than 40 recipients
- pass: awards beyond the escrow
- pass: select
- pass: a second selection
- pass: milestone out of range
- pass: someone never selected
- pass: release milestone 0 to GDLWRQ
- pass: milestone 0 paid exactly
- pass: the same milestone twice
- pass: release with an op id
- pass: the op id replayed
- pass: reputation bump over the cap on release
- pass: claim_prize is not a grant path
- pass: cancel refused while owed
- pass: forfeit milestone 2
- pass: a forfeit pays nothing
- pass: forfeit milestone 3
- pass: a forfeit pays nothing
- pass: start cancel
- pass: grant cancelled

  - [create_event](https://stellar.expert/explorer/testnet/tx/c5b47b3929573b5aca0212e8ae994cc096e6d8996f82d6a87931f61218c37350)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/8bb222337abcbe641feceab0e6cc0a42df9f39f0bfc8fafd8ef0940ecf6bee8a)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/f2ea3b23fc5c877a286bda283fe74f41a89d9bef5a70838143eec2a1a8a19b5e)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/fa5e9402260f73ce57502d9c19f261a6aa754d3e8d3de3bc166577897abc3ecb)
  - [forfeit_milestone](https://stellar.expert/explorer/testnet/tx/eddc904b593ed5fb8a086e7b72d077a2819f251be350a397f744f653f8f7d371)
  - [forfeit_milestone](https://stellar.expert/explorer/testnet/tx/4631e11cca40e28e4f7f3d1af94b9beac96da77ad79ec5e0547e7efe50935bfe)
  - [start_cancel](https://stellar.expert/explorer/testnet/tx/a78f16ad7749b6162ec74fcd00d6ccaabc40251d967b861992ecfbe109c33d66)

## signatures

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: a stranger cannot select (HostError: Error(Auth, InvalidAction))
- pass: owner selects
- pass: a recipient cannot release their own milestone (HostError: Error(Auth, InvalidAction))
- pass: nothing moved
- pass: release milestone 0 to GCKYHA
- pass: milestone 0 paid exactly

  - [create_event](https://stellar.expert/explorer/testnet/tx/52ec37ae2bbf5374fc8f7ac3bbcf438b06e68d4539a2e8a33a9811350a207184)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/89b27a60e009efbefdc39a95d43ee2e2fc401f19c753779cc376c604f231f36e)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/4c16e2307023ba6a7b1a1c4869d48d0f18f859465525f96cc975d050055dc83e)

## manager handover

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: manager accepts
- pass: manager recorded
- pass: the owner no longer selects (HostError: Error(Auth, InvalidAction))
- pass: the manager selects
- pass: no reclaim after selection
- pass: the manager cannot release (HostError: Error(Auth, InvalidAction))
- pass: release milestone 0 to GDNZ7R
- pass: milestone 0 paid exactly
- pass: release milestone 1 to GDNZ7R
- pass: milestone 1 paid exactly
- pass: publish the grant
- pass: owner paid budget plus fee
- pass: second manager accepts
- pass: owner takes management back
- pass: the owner manages again
- pass: owner selects after reclaiming
- pass: release milestone 0 to GDNZ7R
- pass: milestone 0 paid exactly

  - [create_event](https://stellar.expert/explorer/testnet/tx/1d7d450183739e20df6b1be17f4e6356d4078c71c39ee48b41d79db30748717f)
  - [accept_manager](https://stellar.expert/explorer/testnet/tx/8d38e99993dbb6ccafb8bc846f35245dfa42dff3198e5d81c1c8d165f02dc51c)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/360cc3f1833600ed76f02bfd2cab152b63f7c15283322e5b9c931e274caaab1c)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/5f02aaba4a5ae4f1780ed1ed3d21dba8ceb0c729709a04c55529d8085074a4cf)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/34db0f3636da24a5bcb8593b6cb26a92c7b9a17fae345f18dda2486d60a83e71)
  - [create_event](https://stellar.expert/explorer/testnet/tx/4e683964f079e1d429e216286be2b9ca120c57b3a730c03c357c0c0d8811befe)
  - [accept_manager](https://stellar.expert/explorer/testnet/tx/474890e99ca2f1c46b3a38d56c061e94d278f34254abf9bb0bb6ed400f8aa5dd)
  - [reclaim_management](https://stellar.expert/explorer/testnet/tx/aff14140bb4633c3313b2870e3bc7ab78e664607159ea21ec80fe4b64169ce54)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/b2b830153c18e8cc3eb3adfdb2794041787fb758e8803362013aecc1773d48d6)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/73635994c2e7fe49b39d21802d9d9867c81bcddf54800bb1e57d7b9af4a8f261)

## 40 recipients

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: select 40 recipients in one transaction
- pass: release milestone 0 to GBCMCO
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GD44J4
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GATWN5
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCBKBB
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBUOSW
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAN2GJ
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GA263D
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GA7CSV
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAX3V6
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAUHOK
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBY53F
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GC6LKY
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GA6NGO
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCBYZH
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCDIFV
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCSXBP
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAN3I6
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GD2NAN
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDK6G4
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GA4GFW
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBCBMF
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCASCB
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GANYTM
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBWPKC
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDVF35
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBY34X
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAZNDW
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCSEPR
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAGNPF
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCMW2N
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAIYQO
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBLFCZ
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GADYCO
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCERD6
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GD7PKX
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBBJ72
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GB3LJI
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCEODO
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBHR7W
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDN6OG
- pass: milestone 0 paid exactly
- pass: release milestone 1 to GBCMCO
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GD44J4
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GATWN5
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCBKBB
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBUOSW
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAN2GJ
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GA263D
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GA7CSV
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAX3V6
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAUHOK
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBY53F
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GC6LKY
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GA6NGO
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCBYZH
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCDIFV
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCSXBP
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAN3I6
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GD2NAN
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GDK6G4
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GA4GFW
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBCBMF
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCASCB
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GANYTM
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBWPKC
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GDVF35
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBY34X
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAZNDW
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCSEPR
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAGNPF
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCMW2N
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAIYQO
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBLFCZ
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GADYCO
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCERD6
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GD7PKX
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBBJ72
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GB3LJI
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCEODO
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBHR7W
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GDN6OG
- pass: milestone 1 paid exactly
- pass: completed
- pass: a full page of winners reads in one call
- pass: the rest on the next page

  - [create_event](https://stellar.expert/explorer/testnet/tx/e4be5c6f9d78667f6692fa8c88d6bb07d271ad95fd5bc6a2a959d2d15e520585)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/cc425a97bdea34ac4ba598732f796bb376ec09fcdee64b4e126271b7f14783e9)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/8f7420f18531c736cbb8eae8b156c598743b17477463df5801fca86142f8e629)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/89a5f74f1f8ce59b7a83a5d7015564a89bf1e953faf26174bc902ebf53955d6a)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/95cad232dbeec0fbcf28746277a6234df3121d44c313981403ec78b51743671c)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/08b652d0e8b131d28950ddc28938a7cf687bcd3d1e17b3a1c43a9c137d3c6c83)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/0bbe13f5051d60208daaad29a0efd0a67713344e7f607269a34bcaa07f59e2a7)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/c39aaba1a32955115a6949b3e2886d4c788be89ac91d8ded8217f9ce0473e1b7)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/c6ede712ae141d32b5ba68f44b2e24f428a01203e0452394ec1f8742a306d5ce)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/1e9ed4231796963b7ee403eb7fc7dab8a1b44960ed0f505379c7e23c15a67e5f)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/a44097e41b9ca93ec11d62c1b711e6dbeb0bda340159635c58ca7b811c1fa942)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/cc394a81a313af273af6cdd78c28c6a3df5c6d02a15db4f3a0180dab4e343910)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/eec39b5d19dd9c4a9e64ca3cb141e14de359512041d415becc2a22d5127fc75f)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/89af808570c33a27e946e417873f8f30c394ebf861c68b3a72c1ea9f7c9d29c2)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/6ae5e4b1e2a5667e96abca614c331b6ebd17158cb66c4c0a316fa6e6bae7f9fb)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/95e2f92bc7d4280ac40c54bd81abd4e95781c696663287e8ba6f79eff784f07c)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/6a395043baca5fc4d59b65a780f26a6c20a2c0436839caf5294bd8b617b8b5d9)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/433ec596f05ddf1c74de83d5021417cdcb580cd66b25929ea3c1dceb656fe35c)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/771c4926b3e24100d874c4876e788bedf696f60eebcd4e3e035e64f27a783eb4)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/a164215722d5ee1a9471a8377e16baacf8e559929f8115d9910c293c4217a231)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/9184da4e7d63ff2e45e1f1826038c49179b66eae6252ba9e21acdea1720019a6)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/3bec99d55d34530aebf9ffa9ade3dceff5593e55c0cd0542c092b71c1d989a59)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/dbd9a6cfdff6609a1853e16e37b8f710f5baa66be19404101a9c9afd83d03f55)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/e229b5db3849fd670c0c5aaabd497060e02a3f8f55819698e776707011eb14bc)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/898d96f24d298a8b207a25d92012caae5406d43c674402cc1ebeaa9dff5a502b)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/a63f1d35566136a448788fa5c92f98ed519f8b15a0859f09a66fec199b69777e)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/dbb0955f6ac1a5e49d3c4008b36352993d1bb020e7f06e4f8ca85a5dddcd528c)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/8be9de3c2f7670e42b0177af00701028ed2b533c661142b96615761fc644b928)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/aa97c88f7172ef996ddf3e5a8954acaec62d06b31d9476f7c7e7fe3f8d962f8b)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/bfd21b6446ecfa35ff7f384a7fafdaf303065b542fe46170689eaf1fff4674de)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/720f1ae1011629be8f732a2789d7e9c7c6eaf9be59c1314d2048dc9243732878)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2c6ddf6e3d8f6f4b925276239fbbe84db9793d962ecae4e33b33397b4bcf31c3)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/0e70202fb188f7e8d9ff70718867069da992be288fefbc37103c83d507616575)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/1cd47f44747d43b8102adc09253c47662ba95117aa8e79c30bbe24b6de26a5b9)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/3b0c563329068af8db27bb4367d7a00311c2de3fcfe526440d10a19894a294da)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/a24b294b0541abd99f102505c81e06605acef6227cfcdc2a2658e1d3c2eee500)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/1192068375a439115d4fc3de35c1d61b3033074674a04623e21244ce3502469d)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/b0669d5606a0fd80b87c41b2393ef2f5756dff8718643fd0c447ae644786b992)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/d43e3093042feefea227500c01d5ca9d78ac172526e712629cc1640b3202d9d1)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/623d556799d8b1ad746a797dd87e8d0cac84184dc205ec7179ae566e87e31c69)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/4a0a6ec128d4c0557a7c020368737782b3e9a742a42f6921343e7c41ebd2d379)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/0bc4d59a3254996be7f16ea7442d1d078fde587f1c36c568c2315a4d83072f86)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/bf75116941d8b5d9a0093855e990f86e84280e82d168a555d03e9b2946af6708)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/072ec4a7327af95445cc03c1e35ec74e12bef08c4259993ba92175d0b63dfd98)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/43e01892e03dd0859541ed818eb1a6492668f530d2d6a67bbd9c120466b46277)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/057f07aad13f6c7e2267e07eaaa92f4a75da8397b52f04104b016f3e149f103b)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/bcd512c5c05e84a42bf1b7e697db6673be92e3b034e82f9d3146224234506459)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/1f8bcce6a548c7cb1abd594b9f9cde41e2ad9d701b88f56dcd69e6a7e8b9d949)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/bb0f994441a3029a433f9e5fe3925a49830d87063693f228ae2a7f759c67103b)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/38db46a638c18551f2387b13b78d14b03c51efc2f03987806783ab780f584ef0)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/45db69e1f2ca18766d8169d598992ccda93543156ed0183c8c3543da719a211e)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/51c6ee92cfe4b031a50a033da8cf4a33096e7c36f148bcae8f9d93735c9d9ba2)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/85623d3340b3a82be84ff41b1f3e93da601eae4072c86351aa77f5057d3934e5)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/8ed6fedef9dcd9c32a6e3be15a29bb715303132e5a169665d0867827845d0e49)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/47c359f3d09dfd15b10e88faf40297af0590ca9c731376c3ce9de334dbaad6e7)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/5279e9499b57063d866250aed37dfe48b95ddea15f95eb59a9c1e4a8e7420f72)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/6323583e3cfaa1b65fe9c4fdab1b90a4309a9ac88702528840536f880f9a4127)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/1aa6d2908b32288fa7b0785e9da94555d38172bc2db8c3adc6227a78a619cb39)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/09aeccdd2ac50a2cb62f71b98b9e2c424f4128cce575af66544c0f452ffc3044)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/ce16267f65638e973d6a2979c32a38d55ce61acb9a5f7dd0b999ab9833de8e14)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/dd443e6b3dcfa42c28bdbca18f306b853629657d3bd597325b34b6b986ec9db4)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/23008c7c3f4fdcf2c2cda2ccd0d23a9cda9d43de1b1a938845f4fb5cb4daedc2)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/af6b6dd105d1cc3a00d0c45d1d8a4175e7538c19848358f0886196f64fdc9aa4)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/95157cd2849cad9e19af6728428a218c805ff8f9c80cb9beac4472c37fa819b8)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/ca0b6ff1380a37d682f0e08fd4eb385c7af3703afeb5b6e4bc972d63d105ce53)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/629155e6eb22dc9273622b14e7ffac03c254e6dd02835152b6a1924c27401f72)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/9a4b2ea770431e13c9aed2f7141f7e71fead59a469d147dc560e11a2b3bb7331)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/5963df5d2749365d8d11af3978ad1e5b73f1ea011ed9bb4f5b4f289c1c6e475a)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/00c9a40db36e0d0a653ed8a468616d08499fea727eb494c15e3557b9527bd2c5)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/a3b22acd80aeb7d44a018d61ae39ed1b337c0506613e772a89e85da150ce8476)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/eb04dcdb1fd78621961aab5b53545f099167ab149ea5131b9c5c78fc13bc94b6)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/ae11fac683c51fcd105c07b96e83aa27ccebcc202bfc60b1b0601f0d99b1152f)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/5644fa09eb446a71e274695a20a8ba80402acec3d7865892b7a75690068c1a2a)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/f7ef6c5c0eb8b539bdf140b548e9005708a446fe320dd4072ed3e966c1348747)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/d408529208c25f55d57b08984410986a2d0569e8039af0751ec396701d918797)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/0dbdff654e957da7ddcced6dd7d6a7d3165538c6f612bb945f3ea01aa79b47a0)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/e6794ba95b074fa2acf88192d46bf91010363ca40cd1f8c1d634941e14c0b969)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/3bb64aae59bb131c897a01f72a4ad34e99e841b7d2933abbffeedbe536b88970)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/82f8ee770495db7a77ba1980479ee39abda480896c888686e2e8b60d6cad81b8)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/f73fd87258335672c62d30e41c952c6f589af0b9f2738aad28b299dfe3707735)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/8bf0a1e8b632cd0e0e2f035af8785017bec7ce85bd5346c87368ad3d14bd5dc8)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/3597c86c885b31224ae3450461249b0bb275f864dc62a917e7c478cb3672a5a4)

## crowdfunding release validator

- pass: the validator is appointed on chain
- pass: publish the grant
- pass: owner paid budget plus fee
- pass: a backer pledges 60
- pass: backers pay no fee
- pass: the creator alone cannot release (HostError: Error(Auth, InvalidAction))
- pass: release 0 co-signed by the validator
- pass: creator paid net of fee
- pass: fee withheld at release
- pass: release 1 co-signed by the validator
- pass: creator paid net of fee
- pass: fee withheld at release
- pass: campaign completes on its last release

  - [provision 1 accounts](https://stellar.expert/explorer/testnet/tx/453e73dc6472c73cd0772804db0293d5f696a514463ec23b84bfd72be2a56d94)
  - [create_event](https://stellar.expert/explorer/testnet/tx/5a181104d0f181a00c5ee092412b72aca0d75429eedab8cd33d550f961f0cb78)
  - [add_funds](https://stellar.expert/explorer/testnet/tx/580e6f1b46991856acfb48981ea919f364a88b9f9fe2e55acae06533a9d83ad1)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/4082e3ee6f17ff257f00f19af75ae829917140a85ecb3a7eca8fe29b507a2d5c)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/051b276bbf8b5fdb60cc9ec9c6ec7db831474f1b26c35940a6cd019efa05a038)

## fees

- pass: fee account received exactly the fees charged

