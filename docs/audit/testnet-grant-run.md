# Testnet grant run

Run 2026-10-03T18:06:56.506Z against `CBEODVJGUYCIYTVXD7KI5UG3BJ2UE4T7AGI2TGY3T4Q5GQRFGTRYVTZP` with the BGT test asset `CBEOUEMDTM56NRTOOSXCPGMD33RPD5YWEKJ542HAEVREAYCC4QIZ3XIU`.
317 of 317 checks passed across 154 transactions.
Highest fee paid by one transaction: 9998776 stroops.

## setup

- pass: BGT CBEOUEMDTM56NRTOOSXCPGMD33RPD5YWEKJ542HAEVREAYCC4QIZ3XIU is whitelisted on CBEODVJGUYCIYTVXD7KI5UG3BJ2UE4T7AGI2TGY3T4Q5GQRFGTRYVTZP (2.0.0)

  - [provision 2 accounts](https://stellar.expert/explorer/testnet/tx/7c685d046acea0cf323b8d9868b8fd93d3c6ab5b37c6fb192967a622eef88c1d)
  - [provision 4 accounts](https://stellar.expert/explorer/testnet/tx/df6af281c3087efb852e3cdce1c15f0a82491bbd33fac16fedce097898ae7596)
  - [provision 15 accounts](https://stellar.expert/explorer/testnet/tx/e679f186cdbf2b9d472970a3bbdc778cd1641f66d3f3e69f6d8fd587f629836a)
  - [provision 15 accounts](https://stellar.expert/explorer/testnet/tx/a5eb0c85f7679eda38fd060f094ca7d667d84b3c359a8c51ce61d1011b56523e)
  - [provision 15 accounts](https://stellar.expert/explorer/testnet/tx/22b2e9c031c404069cb181e3d45f94f2340b47c3dbfe5d2b7ae7609baa62c3d4)
  - [provision 7 accounts](https://stellar.expert/explorer/testnet/tx/131ef06179e414362d2085e2271c957227cdc98d4b7c059a85668924e92c1218)
  - [provision 3 accounts](https://stellar.expert/explorer/testnet/tx/ecbc1097b28dabd8d9b60177bc605049f88209dffa20466909940fb5983123df)

## micro grant

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: select three recipients
- pass: release milestone 0 to GAG3YV
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAUSXM
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GD4QY3
- pass: milestone 0 paid exactly
- pass: completes when the last award is paid

  - [create_event](https://stellar.expert/explorer/testnet/tx/bff76c4f7acc7f98760865e403ed5687d78733f23e1c785348ce7b5a86b09e3d)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/167ea947488e4658b5eb88267cbc7608d2826f4bd8e16b582018454cf0132b07)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/6f738b752a546afb8d0e9e60651dcdf6752476c2001dca25b534adb48804abe5)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/66f5b15d0463c72209a55715ddbb2d704e36e92787bdf804b008e5b66cbe8c19)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/c1628333683853e479fd1ace196bfd3095fd9a29f8e90a70ef4f5eeecb6313a8)

## tiered, rounding, any order

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: select three tiers
- pass: release milestone 2 to GADYBG
- pass: milestone 2 paid exactly
- pass: release milestone 0 to GBRG5F
- pass: milestone 0 paid exactly
- pass: release milestone 1 to GD3WIU
- pass: milestone 1 paid exactly
- pass: release milestone 2 to GBRG5F
- pass: milestone 2 paid exactly
- pass: release milestone 0 to GADYBG
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GD3WIU
- pass: milestone 0 paid exactly
- pass: release milestone 1 to GBRG5F
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GADYBG
- pass: milestone 1 paid exactly
- pass: release milestone 2 to GD3WIU
- pass: milestone 2 paid exactly
- pass: tier 1 received its whole award
- pass: tier 2 received its whole award
- pass: completed

  - [create_event](https://stellar.expert/explorer/testnet/tx/f9d66a52d1ea981a64d80b7c71a321429207a0895320358d8968a8c9e56c3ff2)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/f48ced0aa477e07e4fb68c743ef2595ac6a87af92c82c04e2306b45ac0f2825e)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/3734a44f17950b4a8a65ed0ffdb5fc5c308f54f13cb94c3dbf97b9a69f35c585)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/59374c0a23b51075a581d2c5e15fa407be8ef95e605013754745cd0ee7f54cfa)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/e5c2ba47fc881a8ddc1f39d0c4105a9aa3bd214338611d1f0b867456310aad6e)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2f0c2eea34a9833b85d9049def3bcde4d463f552c6b17b49af6dcc2fd0f89a76)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/49941ec16dd25620c7b9c4f5a2d61ceaa622606e85e5af558035d4eaab0beafb)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/f2176dc27c1a6a36e7f7036e4382146d7e7b4a3b4ca93c96b44ecbf2fae6fca3)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/58f7cb213f7f31195275af96ec11d1e5ff174ef87ac691e05003c0928bbd91d0)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/288f75d577fd7c9fb0531f7123e73e7c24da90b4dd9f4daccc01fe222f61c30a)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/84af8971fc063d65b83f37ff40afbe7bbc1dd6b045a05eb567c9ecb30f838468)

## forfeit and the cancel guard

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: select one recipient
- pass: release milestone 0 to GCDARJ
- pass: milestone 0 paid exactly
- pass: forfeit milestone 1
- pass: a forfeit pays nothing
- pass: cancel refused while an award is owed (AwardsOutstanding)
- pass: release milestone 2 to GCDARJ
- pass: milestone 2 paid exactly
- pass: start cancel
- pass: grant cancelled
- pass: forfeited share returned to the owner

  - [create_event](https://stellar.expert/explorer/testnet/tx/b87715e8887699381c8cbeb63a93da82646dacacbd841c9960aac0454c1d3fac)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/088d8bfcd1597438d521b2dd641077068693b4de6b9a1e1b07c6f261b589d9dd)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/a430fc40b967d16bd8633bd7b24ea73cefc91bc21306d5a2f53eee12641b5e46)
  - [forfeit_milestone](https://stellar.expert/explorer/testnet/tx/b96cfc4aeaa0f024b4a608fe9dea8dd0fc6ffa268ec4acd6eaf42cedb245f10e)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/56a09ed1691b673f645f491a3ff62f4db9d7f048855df0481c7e72b0f9cbeabf)
  - [start_cancel](https://stellar.expert/explorer/testnet/tx/2a0d5145604453db88e2a9fdca3269ab588044c4ff2f80c14a54c1065ea580eb)

## partners, pro rata

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: partner adds 20.0000000
- pass: partner paid amount plus fee
- pass: award more than the budget, funded by the partner
- pass: release milestone 0 to GCA65U
- pass: milestone 0 paid exactly
- pass: partner adds 15.0000000
- pass: partner paid amount plus fee
- pass: release milestone 1 to GCA65U
- pass: milestone 1 paid exactly
- pass: the late top-up keeps the grant open
- pass: start cancel
- pass: an empty refund batch is refused (InvalidBatchSize)
- pass: anyone can crank
- pass: anyone can finalize
- pass: partner 1 refunded pro rata
- pass: partner 2 refunded pro rata
- pass: owner gets nothing when partners are short

  - [create_event](https://stellar.expert/explorer/testnet/tx/d629631e613016bf786b8fd5844ece081896ce62c4bd472ec36b8c8ce83c9a8f)
  - [add_funds](https://stellar.expert/explorer/testnet/tx/befd68dce6d058fefc0da9000d8956f38b0e7cbbba19f3fb88b15707f8ef55b2)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/058076ba208b7c625022d04bf248eaadb7a1a08113491b72b520fb9b50ca7aba)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/0bfdcaec66e7f2ec136f0ede9ffbfddad9e173afdeef02e6e38bca19bf016b23)
  - [add_funds](https://stellar.expert/explorer/testnet/tx/ce7af8bdb4ab3d27e58c3231e94264586041771fcfbd1c262be12d4bbd1d36b6)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/7b92dbb0b5279e4621b140d4eb526771d7ac4a674f5b534bf0907a0df5e37296)
  - [start_cancel](https://stellar.expert/explorer/testnet/tx/a14d2884832b9eb53293d4d069606acddf0b7d2770ad2141a2d19c53349ec2de)
  - [process_cancel_batch](https://stellar.expert/explorer/testnet/tx/f0178dedb3f92bc9cd7ab4b6dedcebfd286aece1bcfaa5d148daaa0b135b1bdb)
  - [finalize_cancel](https://stellar.expert/explorer/testnet/tx/bdda49e0394d31de6788b8a01591983b9eabd9e3742202e3167f6c608263123f)

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

  - [create_event](https://stellar.expert/explorer/testnet/tx/da2a1869751d7da5014ab4518efd2d0f7f898fab700be9582a3d49541e5c30fa)
  - [add_funds](https://stellar.expert/explorer/testnet/tx/51647f68ee81d423b216a10ca0adab36da2363f3c446423caf23c3190aeff9a4)
  - [add_funds](https://stellar.expert/explorer/testnet/tx/0b17ad6afa05b1685ca34bfc2a43061670c147a82c05579f30e3d26015b14eca)
  - [freeze](https://stellar.expert/explorer/testnet/tx/2bb14ddd42f0e93a25a524857dfa998e4320ecfa1d1acfbaa0701779a1bc9b21)
  - [start_cancel](https://stellar.expert/explorer/testnet/tx/94a1a9fd22fb3d687e6338bc9644eb0363ed9ff6e3c4bf83ab47992d8553979d)
  - [process_cancel_batch](https://stellar.expert/explorer/testnet/tx/39af2679ca4f07ea5b4ab4a26bfa8309ac8e413aedddbef70c706f45a341585f)
  - [finalize_cancel](https://stellar.expert/explorer/testnet/tx/28289796ee0f4fad1bf61fb042539ce79559f2bbfa216ca4d8a13826861a4a0d)
  - [unfreeze](https://stellar.expert/explorer/testnet/tx/d9bda9b42038fe3f0d8141a417369394089571239f3058ea00be6ba7331eb04f)
  - [claim_refund](https://stellar.expert/explorer/testnet/tx/05bb0a7d788d1dd5b754f29b7db53a03141b3a1b743b50fe9dc90b5e406983a2)

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
- pass: release milestone 0 to GAFWBO
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

  - [create_event](https://stellar.expert/explorer/testnet/tx/5dc59e21f7f6049c972687c72969e83ae3dff443eec51b5554af0a69f6e7fc13)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/73b61ad9ae46013e6b2543a5c0356d2a65109d1947ac80de302b284a03b84ac5)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/09fdeb0bad95ca16efdf3925e3872abb5d892c81e30e1be475d0fc6445804d7c)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2cd16bf9ace33ecdd122560f19c01f596d909471fc5e839a14e0447d965274af)
  - [forfeit_milestone](https://stellar.expert/explorer/testnet/tx/fa7e0123246123aa395bf32ec86410a43b75ffa5e864885bcef3a8e3007e4573)
  - [forfeit_milestone](https://stellar.expert/explorer/testnet/tx/310e316b7df97b9153086f89a8f94f051a81d3b21d592ef86f0cf8c5b893f4c4)
  - [start_cancel](https://stellar.expert/explorer/testnet/tx/7671a2264d61c708a6b3dfc7db00cd9e2f5a5508c5f9ab0264ff0b8ab9805274)

## signatures

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: a stranger cannot select (HostError: Error(Auth, InvalidAction))
- pass: owner selects
- pass: a recipient cannot release their own milestone (HostError: Error(Auth, InvalidAction))
- pass: nothing moved
- pass: release milestone 0 to GAZ3NU
- pass: milestone 0 paid exactly

  - [create_event](https://stellar.expert/explorer/testnet/tx/709043ae8556bf6e7251084145aeee4999fb9ca131d9395a0f3b997f961975ab)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/553dca78a0d8f03e9c3944789e9256848f8d646e99dd717c832a6a44701171b3)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/a5b71774cb923e9b380892e533cb38341b5c59d629a48500b9bacfe45cdef7f6)

## manager handover

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: manager accepts
- pass: manager recorded
- pass: the owner no longer selects (HostError: Error(Auth, InvalidAction))
- pass: the manager selects
- pass: no reclaim after selection
- pass: the manager cannot release (HostError: Error(Auth, InvalidAction))
- pass: release milestone 0 to GCCCB3
- pass: milestone 0 paid exactly
- pass: release milestone 1 to GCCCB3
- pass: milestone 1 paid exactly
- pass: publish the grant
- pass: owner paid budget plus fee
- pass: second manager accepts
- pass: owner takes management back
- pass: the owner manages again
- pass: owner selects after reclaiming
- pass: release milestone 0 to GCCCB3
- pass: milestone 0 paid exactly

  - [create_event](https://stellar.expert/explorer/testnet/tx/4bc695b5acac2246ffbb9f13155bf8ad96bea25862d4a6c5a9753d8273f7d8ea)
  - [accept_manager](https://stellar.expert/explorer/testnet/tx/2acc3221c0f0dd006cc167d1d46eaf0f66581a439da9beb4cc98aa0f1ac7e28c)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/ce962cea51d9ae1af7ef8e711e6ca725e026e33e2169ba72720ece0aa0f0e703)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/d865f61ecc1ef0d7e902e6c9f736a25810873aac36af7217b4b6bea94c1348de)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/697337e087807d3d1977d3cc35dcbaeaf5332fda6ddaacc0e624421ffdf04f10)
  - [create_event](https://stellar.expert/explorer/testnet/tx/8a3f9f325d900eda63688cc3799f428ce6b350c9443f30cb9261d4d8abe579de)
  - [accept_manager](https://stellar.expert/explorer/testnet/tx/87ced2faea51a5addf75fe33346cf42f1ad597bf4ae87db5aa5220b2e31595d3)
  - [reclaim_management](https://stellar.expert/explorer/testnet/tx/59db16a6c43d1004aa6bd9ebe62c9d5e72375a738c70aa29c9aa93eb67d2bd1e)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/912f4c9a102b0a7a6d1422dfb394043654478e8912c9e91a0c673351bf49e133)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/bf89b4b16223d9dfe0c5de752be31630098291e6fa3fa463194b27d41c8e335b)

## 40 recipients

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: select 40 recipients in one transaction
- pass: release milestone 0 to GCMQHP
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCZOKQ
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GB5XFM
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAUXJN
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GA5BQH
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCC4AW
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDBJKE
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBXCQ2
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBDFD5
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBASYN
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBBJZJ
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GC4X7Y
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GB5334
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDMPEB
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAJZGD
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCJFRI
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GASYF2
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBO5RV
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAPMBQ
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAICKT
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDLXNY
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GANKX4
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBRWFB
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCQJUK
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCSX3H
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCBO72
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBBT2W
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GC3Y4C
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDL5WI
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GD35UM
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAUT2V
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GARB66
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCDVP2
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCMMO3
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAL6XX
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GB33S4
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAVQFR
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDODL3
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDUONY
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCFFIF
- pass: milestone 0 paid exactly
- pass: release milestone 1 to GCMQHP
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCZOKQ
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GB5XFM
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAUXJN
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GA5BQH
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCC4AW
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GDBJKE
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBXCQ2
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBDFD5
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBASYN
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBBJZJ
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GC4X7Y
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GB5334
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GDMPEB
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAJZGD
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCJFRI
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GASYF2
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBO5RV
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAPMBQ
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAICKT
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GDLXNY
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GANKX4
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBRWFB
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCQJUK
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCSX3H
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCBO72
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBBT2W
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GC3Y4C
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GDL5WI
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GD35UM
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAUT2V
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GARB66
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCDVP2
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCMMO3
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAL6XX
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GB33S4
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAVQFR
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GDODL3
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GDUONY
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCFFIF
- pass: milestone 1 paid exactly
- pass: completed
- pass: a full page of winners reads in one call
- pass: the rest on the next page

  - [create_event](https://stellar.expert/explorer/testnet/tx/5b3621a0c46f37574e4be924fcca449a8f80eadb98fecce00d1daced47fddf68)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/a87168e3ad6d51a5eed768a4aa24da438e3ecedc18b886ca8fb0fdbfc3acaa36)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/23f8d6077c0354614d93873392cf9c30c870d3ebca7e15a08b6d4d5a544557bb)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/d5287744777a577c7dbf21150ece30d9a21eb49d98c897c8a28328b22cec2709)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/04b7ea265a2928c71b04337599460c79c65d05e5b52f0917dbf274cb4109a3c3)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/05a5e6a0ba310287f7c11cfd0528fc5eea356079e8b25687ae71ef7da44d91d8)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/f878750ae8426e5732811e15f2c222287ffa7c156ad4be001e36f2883d08101a)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/dbb23b6ca88ebfd9a29603b582004788d1d7ea3b2754afe49227557c28c5a12c)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/f6d52cfd80428bbd94b9998dcb376579e10d98e7ccb7aa80a017d15f6c4b4309)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/746d35a2f00d9101bc9d5b9926d20d88028d435363f0a47540d355e3efe5e8e9)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/eb92fce7d6b6d745abd04c4f29f04b6cfaea5fb45fd09ad26da56ded801eacb6)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/0ebb79d7b968e8dde8575c90599ce1c8d3433221cb63f2c375e65bd9d18a8b46)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/67a972d25a70ac897688c1f87aed5e717c645e3959132a62e4072cd5ffbde2cd)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/63ebece9126c8f049b74c9cf239d2a0f1955c59d029c4f010c400c7203c9a904)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/6ac1b605783d592109ef3dd6dd453e846953bcaedea890d3aa1409c85708ff75)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/65e8093bdd02fe513df63533adda74d66a277a9c0f707fc35d3b1b5e4916a0c3)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/1743c96061be4b6e74b307d62d18b25ffa0c03cff2b57e2ff06192894fdc7731)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/6358c78d14886b5c610f5df1fc05d2527d709dfdc20ba2700e275b47492375b8)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/28d624ec90efa19b15bc1b31304aae9cf265f0bf5b70df2e912cb9a3cb013f99)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/6c7854c001b7b71be35e59f465d44e379637c58f15223b437f0deef40de1f05c)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/c2870ad5bc9e993110b3db04d661c2a0a4f19c0d68dc1ae02701287914e055ea)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/b1bdc179b28dd40f425a303982d4e386cf0c665cb596bc77ccab99add2b56f54)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/3d83a45d747cde093c5b6af8c10a07949021ea63fdaa1ac7d4f74110f0425b74)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/ad7c147f92d25d9190188240dc3c1cbddc7e56fccc177ec7aedb6f4b25e832e1)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/6e3a6a4c9c9871b6dc0575a59b7ded1fc4d8aa94fefe45a55037665286986d68)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/17de75246c2c8e9890b1099674f3493caea9ea5f11e9387983acaea41b2ebf2f)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/d90b836ac725750e8bad15671c90f9fd8e1a2c389c4853a50f4cddfe4365078a)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/811f8d3607bc37d33d098105dbdc64f0bb53da5cb91b4f9baff7e2a05ee6071f)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/1350bdc035bb5a0bc9af4d68ebf0a561e8a1ed738fcd5624204faeb95ea7432c)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/824713f30cf0640738f7a2a02265f4a079469803d316a38b0ed9aa64cc42dbad)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/b95cd2b1a3234530539d5e690f82e9c95dfb9c47335f81fa3208d0531c74c18d)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/9310b3d386ad03004d6bcf68944ccb101d643cdb30e5069cca5b0ec38140e334)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/de59a4bec1ecf311ed1c8b06d687a9a0c945a957d78635c575f6660f453586d0)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/eb5006e77c8f810771a0a3b4ff703d69414ffb8b7b181ccdb734e873d8d06646)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/d5faf0c3fbbffc7dfbba737da93d1e08b851504f66d3ae279e5d9eb82bbc92c0)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/39ade78a649b43d6225b876fb82563c1bcc416d51b1d4b5ab75cfb721e964d85)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/dbce262fc6969bb486ea7f9f15119dcce23a1ed389f654a08eeae666eb230408)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/3aa5ef54fd80098a01309010c228f8364cd878151be755be21ff05061de6ee29)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/270d23e193bf35fb73401172e9e65c6aafb4920a41c66cb07af2568bbc33369e)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/4dfdab6b8d1872fd20d547fd75892bbef1ba875186c6f76e9ce3bea058c97193)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/22a6adcfdb4d343bb4a71e6c56e88cf2af4eea31e835525fd5b92588f31b26e7)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/0ffe6850247a8951e37c4b3e87f807a1e903c1cd9aa2cde609b1c51d69b2faa2)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/e9fec6d55482e6a89a406dbf7721d0f652344952730b7751a45326b51da87866)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/9058e452d7338004c7ff5d4df51522063cdf3043fe0427c31ba2ca346155aee0)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/a4c713986b33bc17dc575d85868a93019ec94c622382acd204e9c49d2330f53e)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/82881a31331821c8fc4ecb5d6d93756c4c16734c4522019a51390ee8cc2f42b4)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/b5463bb20ffc7f5f5f0b890b33898cd1c7bfffd4983a8b333c42d22b24824d69)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/5542deaf56b44093225375f148bac5a3a6eb3835698ec460ae9ea130e02d1a65)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/95f94c1f6a69a45988aeeecadaa757862094d90834c185071c35a729f1f151f2)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/1b014aa0fad6d6e48c9879aae9bbda30a38644df027280b4ec6e3463596370fa)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/42608bb2260b00d07792a2deafa23889bfb8e24f904fddb4830f92dc56a80b3e)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/4df0343587573126f6088340f42014cc2efdd5de03550258b8808161cdaec3e6)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/c2d7f3fc1bb55ac53d22bd212e1d7eb2067b99e5f05e91d48d9af413b7a483a1)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/af3995eeeed8113bba31779bd9d009b292d6f194e6279405707b380f12057069)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/c51e9a2bf1d79541047b2ab311f2d0e75b4aefe8d2d591eaf7b4d966be646c15)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/a477adeb467f6f99ef72f61adcfba41fcda27951b8e4f488f81b89752088bbf9)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/041c96314b7177614699bebeabb4f7aab43d8756e1b1c8a7102952681433f5b6)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/623fac10464498a668ec6f911fdfbb788bf5f96a5c2184af627f7f1c390a83e0)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/1a0cbf82c2ccebdf58eeed1f8491f7e9a507a73ca02e9407411009604c8c477d)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/81c7e0f540939e1aa757ee58d11e9afdd82c988073506a4027d562aeb90a0dc6)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/691e84aade02c4bbc042a7521e8de2871e5e8116d8cd1ba2c2cc7f495ba8f323)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/932083f17db357ef4e3be21944113e067c65a379e7c10ecb5127fcd05e5a54ab)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/291ef7e0ed995ec29c8a5829c7b2c51895aaac57ac924f3c731e8991c2eab14c)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/07d655308139dd92c6715c16e2e9e384c69d211e1b9b3feea9635b3ada69fad8)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/eb8dadc139cd424087b7945d693c14e1e37a550993d2c883a234ad1716758dba)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/01e8f88b659c150ca4db0953ac18452fde6f9331629243501a2716020aeb81e1)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/bdedc172240a74e9c22171cdca9b7c2acea5430e92361cc48e7572cc7a425ee3)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/4f56d4cf8d716bd8fe2aecd75845d79c00ada2f133e77b853d3b52b6ae9a8641)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/1ae396451987af20fa5d84572fa1367bb583c29df6c1bb53568fd55f75094121)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/02e18ff8fd754609b0beaffcac09a04f2a6947437d6f9ff3ed9ca2c56d0f60ec)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/90dd0b4f772a1c7338a2c48e6cfacd6c9e13d8242f11f2adb9f2d132e78256a3)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/ef389a41dc0fc07104b4db90be90552ad5637f35ebea0373acd573676c270e20)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/8f73ebde17b8dba39522e8418da3ff69a848706a8e8a547fe69622f3590110a0)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/31ff0e1484b51bd9f516e17af47900119f026ff6cf0eb949d99221fe43559359)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/39f2bb3ef0caa38d0055768d8f2d0887fcfb1e0c8d73ea358c3a7f3bb66aa0b3)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2b3d2b2d5e3c5590767920e2187c8da44b2834ffd342d535267dfbd322553f53)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/1d2cb3a8c8f6a677db407fc3c0d8c3a0920e516b4ba1f250317009fb6ea2b1c3)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/7f69276a1c98e739525f41d25fb6b069059ba26d39fce6b4cfcbb17f31fbc33d)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/f423a13d264a35c1f8671c17ebdcc3ce2fc3f4998b70218140f9ed054ac8b451)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/d08345f465bca3379ecc3f425b1b234dcb134d76484ae8f4e19ca38557dcc91f)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/97e4020b7e83f209dc113031f8d80941ea24b4fc3b26cb4fe8e3cc96af067add)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/e26a0edc3fece1843ef312073e3da51d4f2bd7aeb90b702fc7c32111c06fb9e5)

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

  - [provision 1 accounts](https://stellar.expert/explorer/testnet/tx/8a0c5f15009718f71186a8578ca7c0d46c72afe88c2801e36b39f97f8d3dd91b)
  - [create_event](https://stellar.expert/explorer/testnet/tx/6bba582caaa23cd9388a94ef19bf3b526e2c5b4b938e0840c07ecaf0beb3a702)
  - [add_funds](https://stellar.expert/explorer/testnet/tx/679c3042532de989fb5bf5e179b41e0da161f7a0c0ecfa8c43165e8ecebe7a09)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/4b54a1423654e438cffe02278ba8ab5055dede22d1384aa8f8851393123cf83f)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2c0f4f33129245dee815d52fa74f92557c8e942aad5e9ee82666b57606b98f31)

## fees

- pass: fee account received exactly the fees charged

